from uuid import UUID

from sqlalchemy import select, text
from sqlalchemy.ext.asyncio import AsyncSession

from app.core.errors import NotFoundError
from app.modules.users.models import User


async def email_exists(db: AsyncSession, email: str) -> bool:
    """E-mail é único no sistema todo; checagem cross-tenant via auth_find_user()."""
    row = (await db.execute(text("SELECT 1 FROM auth_find_user(:e)"), {"e": email})).first()
    return row is not None


async def get(db: AsyncSession, user_id: UUID) -> User:
    user = await db.get(User, user_id)
    if user is None:
        raise NotFoundError("Usuário não encontrado")
    return user


async def list_all(db: AsyncSession) -> list[User]:
    result = await db.execute(select(User).order_by(User.name))
    return list(result.scalars())
