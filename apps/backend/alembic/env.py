import asyncio

from sqlalchemy.ext.asyncio import create_async_engine

from alembic import context
from app.core.config import settings
from app.core.db import Base
from app.modules import models  # noqa: F401  (registra os modelos)

target_metadata = Base.metadata


def _run(connection) -> None:  # type: ignore[no-untyped-def]
    context.configure(connection=connection, target_metadata=target_metadata, compare_type=True)
    with context.begin_transaction():
        context.run_migrations()


async def run_migrations_online() -> None:
    # migrações SEMPRE com o usuário de migração (dono das tabelas)
    url = context.config.attributes.get("url") or settings.migrations_database_url
    engine = create_async_engine(url)
    async with engine.connect() as conn:
        await conn.run_sync(_run)
    await engine.dispose()


if context.is_offline_mode():
    raise SystemExit("Modo offline não suportado")
asyncio.run(run_migrations_online())
