from dataclasses import dataclass
from datetime import UTC, datetime
from uuid import UUID

from sqlalchemy import text, update
from sqlalchemy.ext.asyncio import AsyncSession

from app.modules.auth.models import RefreshToken


@dataclass(frozen=True, slots=True)
class AuthUser:
    id: UUID
    tenant_id: UUID
    password_hash: str
    is_active: bool


async def find_auth_user(db: AsyncSession, email: str) -> AuthUser | None:
    """Busca cross-tenant via função SECURITY DEFINER (ver migração 0001)."""
    row = (
        await db.execute(
            text("SELECT id, tenant_id, password_hash, is_active FROM auth_find_user(:e)"),
            {"e": email},
        )
    ).first()
    return AuthUser(*row) if row else None


async def revoke_all_for_user(db: AsyncSession, user_id: UUID) -> None:
    await db.execute(
        update(RefreshToken)
        .where(RefreshToken.user_id == user_id, RefreshToken.revoked_at.is_(None))
        .values(revoked_at=datetime.now(UTC))
    )
