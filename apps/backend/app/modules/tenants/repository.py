from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.core.errors import NotFoundError
from app.modules.tenants.models import Tenant


async def get_current(db: AsyncSession) -> Tenant:
    """A única empresa visível pela RLS na transação atual."""

    tenant = (await db.execute(select(Tenant))).scalar_one_or_none()
    if tenant is None:
        raise NotFoundError("Empresa não encontrada")
    return tenant
