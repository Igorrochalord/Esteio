use crate::db::{Conn, Installation};
use crate::error::CoreResult;

use super::models::User;

const COLUMNS: &str = "id, name, email, password_hash, role, is_active, pending, last_login_at, \
                       created_at, updated_at, deleted_at";

pub async fn list_all(conn: &mut Conn) -> CoreResult<Vec<User>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM users WHERE deleted_at IS NULL ORDER BY name COLLATE NOCASE"
    );
    Ok(sqlx::query_as(&sql).fetch_all(conn).await?)
}

pub async fn get(conn: &mut Conn, id: &str) -> CoreResult<Option<User>> {
    let sql = format!("SELECT {COLUMNS} FROM users WHERE id = ? AND deleted_at IS NULL");
    Ok(sqlx::query_as(&sql).bind(id).fetch_optional(conn).await?)
}

/// `email` já em minúsculas.
pub async fn find_by_email(conn: &mut Conn, email: &str) -> CoreResult<Option<User>> {
    let sql = format!("SELECT {COLUMNS} FROM users WHERE lower(email) = ? AND deleted_at IS NULL");
    Ok(sqlx::query_as(&sql)
        .bind(email)
        .fetch_optional(conn)
        .await?)
}

pub async fn insert(conn: &mut Conn, inst: &Installation, u: &User) -> CoreResult<()> {
    sqlx::query(
        "INSERT INTO users (id, name, email, password_hash, role, is_active, pending, last_login_at,
                            branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&u.id)
    .bind(&u.name)
    .bind(&u.email)
    .bind(&u.password_hash)
    .bind(&u.role)
    .bind(u.is_active)
    .bind(u.pending)
    .bind(&u.last_login_at)
    .bind(&inst.branch_id)
    .bind(&inst.device_id)
    .bind(&u.created_at)
    .bind(&u.updated_at)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn update(conn: &mut Conn, u: &User) -> CoreResult<()> {
    sqlx::query(
        "UPDATE users SET name = ?, password_hash = ?, role = ?, is_active = ?, pending = ?,
                          last_login_at = ?, updated_at = ?, deleted_at = ?
         WHERE id = ?",
    )
    .bind(&u.name)
    .bind(&u.password_hash)
    .bind(&u.role)
    .bind(u.is_active)
    .bind(u.pending)
    .bind(&u.last_login_at)
    .bind(&u.updated_at)
    .bind(&u.deleted_at)
    .bind(&u.id)
    .execute(conn)
    .await?;
    Ok(())
}
