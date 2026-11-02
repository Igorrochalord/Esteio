from sqlalchemy import func, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.modules.audit.models import AuditLog


async def search(
    db: AsyncSession, *, entity: str | None, entity_id: str | None, limit: int, offset: int
) -> tuple[list[AuditLog], int]:
    stmt = select(AuditLog)
    if entity:
        stmt = stmt.where(AuditLog.entity == entity)
    if entity_id:
        stmt = stmt.where(AuditLog.entity_id == entity_id)
    total = (await db.execute(select(func.count()).select_from(stmt.subquery()))).scalar_one()
    rows = await db.execute(
        stmt.order_by(AuditLog.created_at.desc(), AuditLog.id).limit(limit).offset(offset)
    )
    return list(rows.scalars()), total
