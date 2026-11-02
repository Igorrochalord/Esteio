from fastapi import APIRouter, Depends, Query

from app.core.context import RequestContext
from app.core.deps import require
from app.core.permissions import Action, Module
from app.modules.audit import repository
from app.modules.audit.schemas import AuditOut, AuditPage

router = APIRouter(prefix="/audit", tags=["audit"])


@router.get("", response_model=AuditPage)
async def list_audit(
    entity: str | None = None,
    entity_id: str | None = None,
    limit: int = Query(50, ge=1, le=200),
    offset: int = Query(0, ge=0),
    ctx: RequestContext = Depends(require(Module.AUDIT, Action.READ)),
) -> AuditPage:
    items, total = await repository.search(
        ctx.db, entity=entity, entity_id=entity_id, limit=limit, offset=offset
    )
    return AuditPage(
        items=[AuditOut.model_validate(i) for i in items], total=total, limit=limit, offset=offset
    )
