from datetime import UTC, datetime

from sqlalchemy.ext.asyncio import AsyncSession

from app.core import audit
from app.core.config import settings
from app.core.context import RequestContext
from app.core.db import set_tenant
from app.core.errors import AuthenticationError
from app.core.permissions import permissions_for
from app.core.security import (
    create_access_token,
    create_refresh_token,
    decode_token,
    verify_password,
)
from app.modules.auth import repository
from app.modules.auth.models import RefreshToken
from app.modules.auth.schemas import MeOut, MeTenant, TokenPair
from app.modules.tenants import repository as tenants_repo
from app.modules.tenants.branding import branding_of
from app.modules.users.models import User
from app.modules.users.schemas import UserOut

INVALID_CREDENTIALS = "E-mail ou senha inválidos"


async def issue_tokens(db: AsyncSession, user: User) -> TokenPair:
    """Gera access + refresh e persiste o refresh. Não faz commit."""
    refresh, jti, expires_at = create_refresh_token(user.id, user.tenant_id)
    db.add(RefreshToken(id=jti, tenant_id=user.tenant_id, user_id=user.id, expires_at=expires_at))
    await db.flush()
    return TokenPair(
        access_token=create_access_token(user.id, user.tenant_id),
        refresh_token=refresh,
        expires_in=settings.access_token_ttl_minutes * 60,
    )


async def login(db: AsyncSession, email: str, password: str, ip: str | None) -> TokenPair:
    found = await repository.find_auth_user(db, email.lower())
    # verifica a senha mesmo sem usuário (tempo constante)
    password_ok = verify_password(password, found.password_hash if found else None)
    if found is None or not password_ok or not found.is_active:
        raise AuthenticationError(INVALID_CREDENTIALS)

    await set_tenant(db, found.tenant_id)
    user = await db.get(User, found.id)
    if user is None:
        raise AuthenticationError(INVALID_CREDENTIALS)
    user.last_login_at = datetime.now(UTC)
    tokens = await issue_tokens(db, user)
    await audit.record(
        db,
        tenant_id=user.tenant_id,
        actor_id=user.id,
        action="auth.login",
        entity="user",
        entity_id=user.id,
        ip=ip,
    )
    await db.commit()
    return tokens


async def refresh(db: AsyncSession, token: str) -> TokenPair:
    """Rotação: o refresh usado é revogado e um novo par é emitido.

    Reuso de um refresh já revogado indica vazamento → revoga todas as sessões do usuário.
    """
    payload = decode_token(token, "refresh")
    if "jti" not in payload:
        raise AuthenticationError("Token inválido")
    await set_tenant(db, payload["tid"])
    stored = await db.get(RefreshToken, payload["jti"])
    if stored is None or stored.user_id != payload["sub"]:
        raise AuthenticationError("Sessão inválida")
    now = datetime.now(UTC)
    if stored.revoked_at is not None:
        await repository.revoke_all_for_user(db, stored.user_id)
        await db.commit()
        raise AuthenticationError("Sessão inválida")
    if stored.expires_at <= now:
        raise AuthenticationError("Sessão expirada")
    user = await db.get(User, stored.user_id)
    if user is None or not user.is_active:
        raise AuthenticationError("Usuário desativado")

    stored.revoked_at = now
    tokens = await issue_tokens(db, user)
    await db.commit()
    return tokens


async def logout(db: AsyncSession, token: str) -> None:
    try:
        payload = decode_token(token, "refresh")
    except AuthenticationError:
        return  # logout é idempotente
    if "jti" not in payload:
        return
    await set_tenant(db, payload["tid"])
    stored = await db.get(RefreshToken, payload["jti"])
    if stored is not None and stored.revoked_at is None:
        stored.revoked_at = datetime.now(UTC)
        await db.commit()


async def me(ctx: RequestContext) -> MeOut:
    tenant = await tenants_repo.get_current(ctx.db)
    branding = branding_of(tenant)
    return MeOut(
        user=UserOut.model_validate(ctx.user),
        tenant=MeTenant(
            **branding.model_dump(),
            id=str(tenant.id),
            plan=tenant.plan,
            subscription_status=tenant.subscription_status,
        ),
        permissions=permissions_for(ctx.user.role),
    )
