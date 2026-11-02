from collections.abc import AsyncIterator
from datetime import datetime
from typing import Any
from uuid import UUID

from sqlalchemy import DateTime, MetaData, func, text
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column
from sqlalchemy.pool import NullPool

from app.core.config import settings

NAMING_CONVENTION = {
    "ix": "ix_%(column_0_label)s",
    "uq": "uq_%(table_name)s_%(column_0_name)s",
    "ck": "ck_%(table_name)s_%(constraint_name)s",
    "fk": "fk_%(table_name)s_%(column_0_name)s_%(referred_table_name)s",
    "pk": "pk_%(table_name)s",
}


class Base(DeclarativeBase):
    metadata = MetaData(naming_convention=NAMING_CONVENTION)


_engine_kwargs: dict[str, Any] = {"pool_pre_ping": True}
if settings.db_null_pool:
    _engine_kwargs["poolclass"] = NullPool

engine = create_async_engine(settings.database_url, **_engine_kwargs)
SessionLocal = async_sessionmaker(engine, expire_on_commit=False, autoflush=True)


async def get_db() -> AsyncIterator[AsyncSession]:
    """Sessão sem tenant definido (onboarding, login, refresh)."""
    async with SessionLocal() as session:
        yield session


async def set_tenant(session: AsyncSession, tenant_id: UUID) -> None:
    """Equivalente a SET LOCAL app.tenant_id, mas parametrizado.

    Vale só até o fim da transação corrente: após commit/rollback, o tenant
    precisa ser definido de novo. As policies de RLS leem esse valor via
    app_current_tenant().
    """
    await session.execute(
        text("SELECT set_config('app.tenant_id', :tid, true)"), {"tid": str(tenant_id)}
    )


class TimestampMixin:
    """created_at/updated_at gerados no banco e devolvidos via RETURNING
    (sem eager_defaults, ler updated_at após um UPDATE dispararia I/O implícito)."""

    __mapper_args__ = {"eager_defaults": True}  # noqa: RUF012

    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), server_default=func.now())
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), server_default=func.now(), onupdate=func.now()
    )
