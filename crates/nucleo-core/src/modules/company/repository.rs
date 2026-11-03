use crate::db::{Conn, Installation};
use crate::error::CoreResult;

use super::models::Company;

const COLUMNS: &str = "id, name, cnpj, primary_color, secondary_color, logo, logo_type, logo_id, created_at, updated_at";

pub async fn get(conn: &mut Conn) -> CoreResult<Option<Company>> {
    let sql = format!("SELECT {COLUMNS} FROM company WHERE deleted_at IS NULL");
    Ok(sqlx::query_as(&sql).fetch_optional(conn).await?)
}

pub async fn exists(conn: &mut Conn) -> CoreResult<bool> {
    Ok(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM company")
        .fetch_one(conn)
        .await?
        > 0)
}

pub async fn insert(conn: &mut Conn, inst: &Installation, c: &Company) -> CoreResult<()> {
    sqlx::query(
        "INSERT INTO company (id, name, cnpj, primary_color, secondary_color, logo, logo_type, logo_id,
                              branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&c.id)
    .bind(&c.name)
    .bind(&c.cnpj)
    .bind(&c.primary_color)
    .bind(&c.secondary_color)
    .bind(&c.logo)
    .bind(&c.logo_type)
    .bind(&c.logo_id)
    .bind(&inst.branch_id)
    .bind(&inst.device_id)
    .bind(&c.created_at)
    .bind(&c.updated_at)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn update(conn: &mut Conn, c: &Company) -> CoreResult<()> {
    sqlx::query(
        "UPDATE company SET name = ?, cnpj = ?, primary_color = ?, secondary_color = ?,
                            logo = ?, logo_type = ?, logo_id = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&c.name)
    .bind(&c.cnpj)
    .bind(&c.primary_color)
    .bind(&c.secondary_color)
    .bind(&c.logo)
    .bind(&c.logo_type)
    .bind(&c.logo_id)
    .bind(&c.updated_at)
    .bind(&c.id)
    .execute(conn)
    .await?;
    Ok(())
}
