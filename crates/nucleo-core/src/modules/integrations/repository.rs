use serde_json::Value;

use crate::db::{self, Conn, Installation};
use crate::error::CoreResult;

use super::models::{Integration, Job, JobOut, QueueSummary};

const COLUMNS: &str = "id, provider, environment, enabled, config, secrets, status, status_detail,
                       checked_at, created_at, updated_at";

pub async fn list(conn: &mut Conn) -> CoreResult<Vec<Integration>> {
    let sql =
        format!("SELECT {COLUMNS} FROM integrations WHERE deleted_at IS NULL ORDER BY provider");
    Ok(sqlx::query_as(&sql).fetch_all(conn).await?)
}

pub async fn by_provider(conn: &mut Conn, provider: &str) -> CoreResult<Option<Integration>> {
    let sql =
        format!("SELECT {COLUMNS} FROM integrations WHERE provider = ? AND deleted_at IS NULL");
    Ok(sqlx::query_as(&sql)
        .bind(provider)
        .fetch_optional(conn)
        .await?)
}

pub async fn get(conn: &mut Conn, id: &str) -> CoreResult<Option<Integration>> {
    let sql = format!("SELECT {COLUMNS} FROM integrations WHERE id = ? AND deleted_at IS NULL");
    Ok(sqlx::query_as(&sql).bind(id).fetch_optional(conn).await?)
}

pub async fn insert(conn: &mut Conn, inst: &Installation, i: &Integration) -> CoreResult<()> {
    sqlx::query(
        "INSERT INTO integrations (id, provider, environment, enabled, config, secrets, status,
                                   status_detail, checked_at, branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&i.id)
    .bind(&i.provider)
    .bind(&i.environment)
    .bind(i.enabled)
    .bind(i.config.to_string())
    .bind(&i.secrets)
    .bind(&i.status)
    .bind(&i.status_detail)
    .bind(&i.checked_at)
    .bind(&inst.branch_id)
    .bind(&inst.device_id)
    .bind(&i.created_at)
    .bind(&i.updated_at)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn update(conn: &mut Conn, i: &Integration) -> CoreResult<()> {
    sqlx::query(
        "UPDATE integrations SET environment = ?, enabled = ?, config = ?, secrets = ?, status = ?,
                                 status_detail = ?, checked_at = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&i.environment)
    .bind(i.enabled)
    .bind(i.config.to_string())
    .bind(&i.secrets)
    .bind(&i.status)
    .bind(&i.status_detail)
    .bind(&i.checked_at)
    .bind(&i.updated_at)
    .bind(&i.id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn soft_delete(conn: &mut Conn, id: &str) -> CoreResult<()> {
    let now = db::now();
    sqlx::query("UPDATE integrations SET deleted_at = ?, enabled = 0, updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

// ---------- fila ----------

const JOB_COLUMNS: &str =
    "id, integration_id, kind, payload, status, attempts, next_run_at, last_error,
                           result, ref_entity, ref_id, finished_at, created_at, updated_at";

pub struct NewJob<'a> {
    pub integration_id: &'a str,
    pub kind: &'a str,
    pub payload: &'a Value,
    pub ref_entity: Option<&'a str>,
    pub ref_id: Option<&'a str>,
}

pub async fn insert_job(conn: &mut Conn, inst: &Installation, j: NewJob<'_>) -> CoreResult<String> {
    let id = db::new_id();
    let now = db::now();
    sqlx::query(
        "INSERT INTO integration_jobs (id, integration_id, kind, payload, next_run_at, ref_entity, ref_id,
                                       branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(j.integration_id)
    .bind(j.kind)
    .bind(j.payload.to_string())
    .bind(&now)
    .bind(j.ref_entity)
    .bind(j.ref_id)
    .bind(&inst.branch_id)
    .bind(&inst.device_id)
    .bind(&now)
    .bind(&now)
    .execute(conn)
    .await?;
    Ok(id)
}

pub async fn get_job(conn: &mut Conn, id: &str) -> CoreResult<Option<Job>> {
    let sql =
        format!("SELECT {JOB_COLUMNS} FROM integration_jobs WHERE id = ? AND deleted_at IS NULL");
    Ok(sqlx::query_as(&sql).bind(id).fetch_optional(conn).await?)
}

/// Trabalhos vencidos, mais antigos primeiro.
pub async fn due_jobs(conn: &mut Conn, now: &str, limit: i64) -> CoreResult<Vec<Job>> {
    let sql = format!(
        "SELECT {JOB_COLUMNS} FROM integration_jobs
         WHERE status = 'pending' AND next_run_at <= ? AND deleted_at IS NULL
         ORDER BY next_run_at LIMIT ?"
    );
    Ok(sqlx::query_as(&sql)
        .bind(now)
        .bind(limit)
        .fetch_all(conn)
        .await?)
}

/// Marca como em execução só se ainda estiver pendente (evita rodar duas vezes).
pub async fn claim(conn: &mut Conn, id: &str) -> CoreResult<bool> {
    let r = sqlx::query(
        "UPDATE integration_jobs SET status = 'running', updated_at = ?
         WHERE id = ? AND status = 'pending'",
    )
    .bind(db::now())
    .bind(id)
    .execute(conn)
    .await?;
    Ok(r.rows_affected() == 1)
}

/// Trabalhos que ficaram "em execução" (app fechado no meio) voltam para a fila.
pub async fn release_stale(conn: &mut Conn, older_than: &str) -> CoreResult<u64> {
    let r = sqlx::query(
        "UPDATE integration_jobs SET status = 'pending', updated_at = ?
         WHERE status = 'running' AND updated_at < ?",
    )
    .bind(db::now())
    .bind(older_than)
    .execute(conn)
    .await?;
    Ok(r.rows_affected())
}

pub struct JobUpdate<'a> {
    pub status: &'a str,
    pub attempts: i64,
    pub next_run_at: &'a str,
    pub last_error: Option<&'a str>,
    pub result: Option<&'a Value>,
    pub finished_at: Option<&'a str>,
}

pub async fn update_job(conn: &mut Conn, id: &str, u: JobUpdate<'_>) -> CoreResult<()> {
    sqlx::query(
        "UPDATE integration_jobs SET status = ?, attempts = ?, next_run_at = ?, last_error = ?,
                                     result = ?, finished_at = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(u.status)
    .bind(u.attempts)
    .bind(u.next_run_at)
    .bind(u.last_error)
    .bind(u.result.map(Value::to_string))
    .bind(u.finished_at)
    .bind(db::now())
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn list_jobs(
    conn: &mut Conn,
    status: Option<&str>,
    limit: i64,
) -> CoreResult<Vec<JobOut>> {
    Ok(sqlx::query_as(
        "SELECT j.id, i.provider, j.kind, j.status, j.attempts, j.next_run_at, j.last_error,
                j.ref_entity, j.ref_id, j.finished_at, j.created_at
         FROM integration_jobs j JOIN integrations i ON i.id = j.integration_id
         WHERE j.deleted_at IS NULL AND (?1 IS NULL OR j.status = ?1)
         ORDER BY j.created_at DESC LIMIT ?2",
    )
    .bind(status)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

pub async fn summary(conn: &mut Conn) -> CoreResult<QueueSummary> {
    let (pending, running, failed): (i64, i64, i64) = sqlx::query_as(
        "SELECT coalesce(sum(status = 'pending'), 0), coalesce(sum(status = 'running'), 0),
                coalesce(sum(status = 'failed'), 0)
         FROM integration_jobs WHERE deleted_at IS NULL",
    )
    .fetch_one(&mut *conn)
    .await?;
    let enabled_integrations: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM integrations WHERE enabled = 1 AND deleted_at IS NULL",
    )
    .fetch_one(&mut *conn)
    .await?;
    let last_success_at: Option<String> =
        sqlx::query_scalar("SELECT max(finished_at) FROM integration_jobs WHERE status = 'done'")
            .fetch_one(&mut *conn)
            .await?;
    Ok(QueueSummary {
        pending,
        running,
        failed,
        enabled_integrations,
        last_success_at,
    })
}
