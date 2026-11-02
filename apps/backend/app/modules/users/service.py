from uuid import UUID

from sqlalchemy.exc import IntegrityError

from app.core import audit
from app.core.context import RequestContext
from app.core.errors import BusinessRuleError, ConflictError
from app.core.permissions import Role
from app.core.security import hash_password
from app.modules.auth import repository as auth_repo
from app.modules.users import repository
from app.modules.users.models import User
from app.modules.users.schemas import UserCreate, UserUpdate


async def list_users(ctx: RequestContext) -> list[User]:
    return await repository.list_all(ctx.db)


async def get_user(ctx: RequestContext, user_id: UUID) -> User:
    return await repository.get(ctx.db, user_id)


async def create_user(ctx: RequestContext, data: UserCreate) -> User:
    email = data.email.lower()
    if await repository.email_exists(ctx.db, email):
        raise ConflictError("E-mail já cadastrado")
    user = User(
        tenant_id=ctx.tenant_id,
        name=data.name.strip(),
        email=email,
        password_hash=hash_password(data.password),
        role=data.role,
        is_active=True,
    )
    ctx.db.add(user)
    try:
        await ctx.db.flush()
    except IntegrityError as exc:
        await ctx.db.rollback()
        raise ConflictError("E-mail já cadastrado") from exc
    await audit.record(
        ctx.db,
        tenant_id=ctx.tenant_id,
        actor_id=ctx.user.id,
        action="user.create",
        entity="user",
        entity_id=user.id,
        after=audit.snapshot(user),
        ip=ctx.ip,
    )
    await ctx.db.commit()
    return user


async def update_user(ctx: RequestContext, user_id: UUID, data: UserUpdate) -> User:
    user = await repository.get(ctx.db, user_id)
    changes = data.model_dump(exclude_unset=True, exclude_none=True)

    touches_access = "role" in changes or "is_active" in changes
    if touches_access and user.role == Role.OWNER:
        raise BusinessRuleError("O owner não pode ser rebaixado nem desativado")
    if touches_access and user.id == ctx.user.id:
        raise BusinessRuleError("Você não pode alterar o próprio papel ou status")
    if user.role == Role.OWNER and ctx.user.role != Role.OWNER:
        raise BusinessRuleError("Apenas o owner pode editar os próprios dados")

    before = audit.snapshot(user)
    if "name" in changes:
        user.name = changes["name"].strip()
    if "role" in changes:
        user.role = changes["role"]
    if "is_active" in changes:
        user.is_active = changes["is_active"]
        if not user.is_active:
            await auth_repo.revoke_all_for_user(ctx.db, user.id)
    await ctx.db.flush()

    b, a = audit.diff(before, audit.snapshot(user))
    if a:
        action = "user.deactivate" if a.get("is_active") is False else "user.update"
        await audit.record(
            ctx.db,
            tenant_id=ctx.tenant_id,
            actor_id=ctx.user.id,
            action=action,
            entity="user",
            entity_id=user.id,
            before=b,
            after=a,
            ip=ctx.ip,
        )
    await ctx.db.commit()
    return user
