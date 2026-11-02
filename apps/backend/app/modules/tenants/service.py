import logging
from uuid import uuid4

from sqlalchemy.exc import IntegrityError
from sqlalchemy.ext.asyncio import AsyncSession

from app.core import audit, storage
from app.core.context import RequestContext
from app.core.db import set_tenant
from app.core.errors import BusinessRuleError, ConflictError
from app.core.permissions import Role
from app.core.security import hash_password
from app.modules.auth import service as auth_service
from app.modules.auth.schemas import TokenPair
from app.modules.tenants import repository
from app.modules.tenants.branding import branding_of
from app.modules.tenants.models import Tenant
from app.modules.tenants.schemas import (
    BrandingOut,
    BrandingUpdate,
    OnboardingRequest,
    TenantUpdate,
)
from app.modules.users import repository as users_repo
from app.modules.users.models import User

log = logging.getLogger(__name__)

LOGO_TYPES = {  # assinatura → (content-type, extensão). SVG fica de fora (pode conter script).
    b"\x89PNG\r\n\x1a\n": ("image/png", "png"),
    b"\xff\xd8\xff": ("image/jpeg", "jpg"),
}


async def onboard(db: AsyncSession, data: OnboardingRequest, ip: str | None) -> TokenPair:
    """Cria empresa + owner numa única transação e já devolve os tokens."""
    email = data.owner.email.lower()
    if await users_repo.email_exists(db, email):
        raise ConflictError("E-mail já cadastrado")

    tenant_id = uuid4()
    await set_tenant(db, tenant_id)  # antes dos INSERTs: a policy WITH CHECK exige
    tenant = Tenant(
        id=tenant_id,
        name=data.company.name.strip(),
        cnpj=data.company.cnpj,
        primary_color=data.company.primary_color,
        secondary_color=data.company.secondary_color,
        plan="trial",
        subscription_status="trialing",
    )
    owner = User(
        tenant_id=tenant_id,
        name=data.owner.name.strip(),
        email=email,
        password_hash=hash_password(data.owner.password),
        role=Role.OWNER.value,
        is_active=True,
    )
    db.add(tenant)
    await db.flush()
    db.add(owner)
    try:
        await db.flush()
    except IntegrityError as exc:  # corrida no e-mail único
        await db.rollback()
        raise ConflictError("E-mail já cadastrado") from exc

    await audit.record(
        db,
        tenant_id=tenant_id,
        actor_id=owner.id,
        action="tenant.create",
        entity="tenant",
        entity_id=tenant_id,
        after=audit.snapshot(tenant),
        ip=ip,
    )
    await audit.record(
        db,
        tenant_id=tenant_id,
        actor_id=owner.id,
        action="user.create",
        entity="user",
        entity_id=owner.id,
        after=audit.snapshot(owner),
        ip=ip,
    )
    tokens = await auth_service.issue_tokens(db, owner)
    await db.commit()
    return tokens


async def get_tenant(ctx: RequestContext) -> Tenant:
    return await repository.get_current(ctx.db)


async def update_tenant(ctx: RequestContext, data: TenantUpdate) -> Tenant:
    tenant = await repository.get_current(ctx.db)
    before = audit.snapshot(tenant)
    changes = data.model_dump(exclude_unset=True)
    if changes.get("name"):
        tenant.name = changes["name"].strip()
    if "cnpj" in changes:
        tenant.cnpj = changes["cnpj"]
    await ctx.db.flush()
    await _audit_update(ctx, tenant, before, "tenant.update")
    await ctx.db.commit()
    return tenant


async def get_branding(ctx: RequestContext) -> BrandingOut:
    return branding_of(await repository.get_current(ctx.db))


async def update_branding(ctx: RequestContext, data: BrandingUpdate) -> BrandingOut:
    tenant = await repository.get_current(ctx.db)
    before = audit.snapshot(tenant)
    for field, value in data.model_dump(exclude_unset=True, exclude_none=True).items():
        setattr(tenant, field, value)
    await ctx.db.flush()
    await _audit_update(ctx, tenant, before, "branding.update")
    await ctx.db.commit()
    return branding_of(tenant)


async def upload_logo(ctx: RequestContext, content: bytes, max_bytes: int) -> BrandingOut:
    if len(content) > max_bytes:
        raise BusinessRuleError(f"Logo excede {max_bytes // 1024} KB")
    detected = next((v for sig, v in LOGO_TYPES.items() if content.startswith(sig)), None)
    if detected is None:
        raise BusinessRuleError("Formato inválido: envie PNG ou JPEG")
    content_type, ext = detected

    tenant = await repository.get_current(ctx.db)
    before = audit.snapshot(tenant)
    old_key = tenant.logo_key
    new_key = f"tenants/{tenant.id}/branding/logo-{uuid4().hex}.{ext}"
    await storage.put_object(new_key, content, content_type)
    tenant.logo_key = new_key
    await ctx.db.flush()
    await _audit_update(ctx, tenant, before, "branding.logo_upload")
    await ctx.db.commit()
    await _delete_quietly(old_key)
    return branding_of(tenant)


async def remove_logo(ctx: RequestContext) -> BrandingOut:
    tenant = await repository.get_current(ctx.db)
    if tenant.logo_key is None:
        return branding_of(tenant)
    before = audit.snapshot(tenant)
    old_key = tenant.logo_key
    tenant.logo_key = None
    await ctx.db.flush()
    await _audit_update(ctx, tenant, before, "branding.logo_remove")
    await ctx.db.commit()
    await _delete_quietly(old_key)
    return branding_of(tenant)


async def _audit_update(ctx: RequestContext, tenant: Tenant, before: dict, action: str) -> None:
    b, a = audit.diff(before, audit.snapshot(tenant))
    if a:
        await audit.record(
            ctx.db,
            tenant_id=ctx.tenant_id,
            actor_id=ctx.user.id,
            action=action,
            entity="tenant",
            entity_id=tenant.id,
            before=b,
            after=a,
            ip=ctx.ip,
        )


async def _delete_quietly(key: str | None) -> None:
    if not key:
        return
    try:
        await storage.delete_object(key)
    except Exception:  # objeto órfão não deve quebrar a requisição
        log.warning("falha ao remover objeto %s", key, exc_info=True)
