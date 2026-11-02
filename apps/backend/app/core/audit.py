"""Registro de auditoria (append-only: o usuário da aplicação só tem INSERT/SELECT)."""

import enum
from datetime import date, datetime
from decimal import Decimal
from typing import Any
from uuid import UUID

from sqlalchemy import inspect
from sqlalchemy.ext.asyncio import AsyncSession

from app.modules.audit.models import AuditLog

# nunca vão para before/after
SENSITIVE_FIELDS = {"password_hash"}


def _jsonable(value: Any) -> Any:
    if isinstance(value, UUID | Decimal):
        return str(value)
    if isinstance(value, datetime | date):
        return value.isoformat()
    if isinstance(value, enum.Enum):
        return value.value
    return value


def snapshot(obj: Any) -> dict[str, Any]:
    """Foto das colunas de um modelo ORM, sem campos sensíveis."""
    mapper = inspect(obj).mapper
    return {
        col.key: _jsonable(getattr(obj, col.key))
        for col in mapper.column_attrs
        if col.key not in SENSITIVE_FIELDS
    }


def diff(before: dict[str, Any], after: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    """Reduz before/after aos campos que mudaram."""
    changed = {k for k in after if before.get(k) != after.get(k)}
    changed.discard("updated_at")
    return {k: before.get(k) for k in changed}, {k: after[k] for k in changed}


async def record(
    db: AsyncSession,
    *,
    tenant_id: UUID,
    actor_id: UUID | None,
    action: str,
    entity: str,
    entity_id: UUID | str | None,
    before: dict[str, Any] | None = None,
    after: dict[str, Any] | None = None,
    ip: str | None = None,
) -> None:
    db.add(
        AuditLog(
            tenant_id=tenant_id,
            user_id=actor_id,
            action=action,
            entity=entity,
            entity_id=str(entity_id) if entity_id is not None else None,
            before=before,
            after=after,
            ip=ip,
        )
    )
    await db.flush()
