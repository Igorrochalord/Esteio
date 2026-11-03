use crate::error::CoreResult;
use crate::Ctx;

use super::models::{AuditEntry, AuditQuery, Page};
use super::repository;

pub async fn list(ctx: &Ctx, q: AuditQuery) -> CoreResult<Page<AuditEntry>> {
    let limit = q.limit.clamp(1, 200);
    let offset = q.offset.max(0);
    let entity = q.entity.as_deref().filter(|s| !s.is_empty());
    let entity_id = q.entity_id.as_deref().filter(|s| !s.is_empty());
    let mut conn = ctx.db.acquire().await?;
    let (rows, total) = repository::search(&mut conn, entity, entity_id, limit, offset).await?;
    Ok(Page {
        items: rows.into_iter().map(Into::into).collect(),
        total,
        limit,
        offset,
    })
}
