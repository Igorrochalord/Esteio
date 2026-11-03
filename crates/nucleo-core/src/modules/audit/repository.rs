use crate::db::Conn;
use crate::error::CoreResult;

use super::models::AuditRow;

pub async fn search(
    conn: &mut Conn,
    entity: Option<&str>,
    entity_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> CoreResult<(Vec<AuditRow>, i64)> {
    // filtros opcionais com `? IS NULL OR ...` mantêm a query estática (sem concatenar SQL)
    const FILTER: &str = "(?1 IS NULL OR a.entity = ?1) AND (?2 IS NULL OR a.entity_id = ?2)";
    let total: i64 =
        sqlx::query_scalar(&format!("SELECT count(*) FROM audit_logs a WHERE {FILTER}"))
            .bind(entity)
            .bind(entity_id)
            .fetch_one(&mut *conn)
            .await?;
    let rows = sqlx::query_as(&format!(
        "SELECT a.id, a.user_id, u.name AS user_name, a.action, a.entity, a.entity_id,
                a.before, a.after, a.created_at
         FROM audit_logs a LEFT JOIN users u ON u.id = a.user_id
         WHERE {FILTER}
         ORDER BY a.created_at DESC, a.id DESC
         LIMIT ?3 OFFSET ?4"
    ))
    .bind(entity)
    .bind(entity_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    Ok((rows, total))
}
