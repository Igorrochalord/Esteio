use crate::db::{self, Conn, Installation};
use crate::error::CoreResult;

use super::models::{Category, MovementRow, ProductRow};

const SELECT: &str =
    "SELECT p.id, p.code, p.barcode, p.name, p.category_id, c.name AS category_name,
        p.unit, p.price_cents, p.cost_cents, p.track_stock, p.stock_milli, p.min_stock_milli,
        p.active, p.created_at, p.updated_at
    FROM products p LEFT JOIN categories c ON c.id = p.category_id AND c.deleted_at IS NULL
    WHERE p.deleted_at IS NULL";

pub async fn list(conn: &mut Conn) -> CoreResult<Vec<ProductRow>> {
    let sql = format!("{SELECT} ORDER BY p.name COLLATE NOCASE");
    Ok(sqlx::query_as(&sql).fetch_all(conn).await?)
}

pub async fn get(conn: &mut Conn, id: &str) -> CoreResult<Option<ProductRow>> {
    let sql = format!("{SELECT} AND p.id = ?");
    Ok(sqlx::query_as(&sql).bind(id).fetch_optional(conn).await?)
}

pub async fn find_by_code(conn: &mut Conn, code: &str) -> CoreResult<Option<ProductRow>> {
    let sql = format!("{SELECT} AND lower(p.code) = lower(?)");
    Ok(sqlx::query_as(&sql).bind(code).fetch_optional(conn).await?)
}

/// Próximo código numérico livre (0001, 0002…).
pub async fn next_code(conn: &mut Conn) -> CoreResult<String> {
    let max: Option<i64> = sqlx::query_scalar(
        "SELECT max(CAST(code AS INTEGER)) FROM products
         WHERE deleted_at IS NULL AND code GLOB '[0-9]*' AND code NOT GLOB '*[^0-9]*'",
    )
    .fetch_one(conn)
    .await?;
    Ok(format!("{:04}", max.unwrap_or(0) + 1))
}

pub async fn insert(conn: &mut Conn, inst: &Installation, p: &ProductRow) -> CoreResult<()> {
    sqlx::query(
        "INSERT INTO products (id, code, barcode, name, category_id, unit, price_cents, cost_cents,
            track_stock, stock_milli, min_stock_milli, active, branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&p.id)
    .bind(&p.code)
    .bind(&p.barcode)
    .bind(&p.name)
    .bind(&p.category_id)
    .bind(&p.unit)
    .bind(p.price_cents)
    .bind(p.cost_cents)
    .bind(p.track_stock)
    .bind(p.min_stock_milli)
    .bind(p.active)
    .bind(&inst.branch_id)
    .bind(&inst.device_id)
    .bind(&p.created_at)
    .bind(&p.updated_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// Atualiza o cadastro (o saldo só muda por movimentação).
pub async fn update(conn: &mut Conn, p: &ProductRow) -> CoreResult<()> {
    sqlx::query(
        "UPDATE products SET code = ?, barcode = ?, name = ?, category_id = ?, unit = ?,
            price_cents = ?, cost_cents = ?, track_stock = ?, min_stock_milli = ?, active = ?,
            updated_at = ?
         WHERE id = ?",
    )
    .bind(&p.code)
    .bind(&p.barcode)
    .bind(&p.name)
    .bind(&p.category_id)
    .bind(&p.unit)
    .bind(p.price_cents)
    .bind(p.cost_cents)
    .bind(p.track_stock)
    .bind(p.min_stock_milli)
    .bind(p.active)
    .bind(&p.updated_at)
    .bind(&p.id)
    .execute(conn)
    .await?;
    Ok(())
}

pub struct NewMovement<'a> {
    pub product_id: &'a str,
    pub kind: &'a str,
    pub delta_milli: i64,
    pub unit_cost_cents: Option<i64>,
    pub note: Option<&'a str>,
    pub sale_id: Option<&'a str>,
    pub user_id: Option<&'a str>,
}

/// Aplica a variação no saldo e grava a movimentação. Devolve o novo saldo.
pub async fn apply_movement(
    conn: &mut Conn,
    inst: &Installation,
    m: NewMovement<'_>,
) -> CoreResult<i64> {
    let now = db::now();
    let balance: i64 = sqlx::query_scalar(
        "UPDATE products SET stock_milli = stock_milli + ?, updated_at = ? WHERE id = ?
         RETURNING stock_milli",
    )
    .bind(m.delta_milli)
    .bind(&now)
    .bind(m.product_id)
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO stock_movements (id, product_id, kind, qty_milli, balance_milli, unit_cost_cents,
            note, sale_id, user_id, branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(db::new_id())
    .bind(m.product_id)
    .bind(m.kind)
    .bind(m.delta_milli)
    .bind(balance)
    .bind(m.unit_cost_cents)
    .bind(m.note)
    .bind(m.sale_id)
    .bind(m.user_id)
    .bind(&inst.branch_id)
    .bind(&inst.device_id)
    .bind(&now)
    .bind(&now)
    .execute(conn)
    .await?;
    Ok(balance)
}

pub async fn movements(
    conn: &mut Conn,
    product_id: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    limit: i64,
) -> CoreResult<Vec<MovementRow>> {
    Ok(sqlx::query_as(
        "SELECT m.id, m.product_id, p.name AS product_name, m.kind, m.qty_milli, m.balance_milli,
                m.unit_cost_cents, m.note, m.sale_id, u.name AS user_name, m.created_at
         FROM stock_movements m
         JOIN products p ON p.id = m.product_id
         LEFT JOIN users u ON u.id = m.user_id
         WHERE (?1 IS NULL OR m.product_id = ?1)
           AND (?2 IS NULL OR m.created_at >= ?2)
           AND (?3 IS NULL OR m.created_at < ?3)
         ORDER BY m.created_at DESC, m.id DESC
         LIMIT ?4",
    )
    .bind(product_id)
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

pub async fn categories(conn: &mut Conn) -> CoreResult<Vec<Category>> {
    Ok(sqlx::query_as(
        "SELECT c.id, c.name,
                (SELECT count(*) FROM products p WHERE p.category_id = c.id AND p.deleted_at IS NULL)
                    AS product_count
         FROM categories c WHERE c.deleted_at IS NULL ORDER BY c.name COLLATE NOCASE",
    )
    .fetch_all(conn)
    .await?)
}

pub async fn category_by_name(conn: &mut Conn, name: &str) -> CoreResult<Option<Category>> {
    Ok(sqlx::query_as(
        "SELECT id, name, 0 AS product_count FROM categories
         WHERE lower(name) = lower(?) AND deleted_at IS NULL",
    )
    .bind(name)
    .fetch_optional(conn)
    .await?)
}

pub async fn category_exists(conn: &mut Conn, id: &str) -> CoreResult<bool> {
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM categories WHERE id = ? AND deleted_at IS NULL")
            .bind(id)
            .fetch_one(conn)
            .await?;
    Ok(n > 0)
}

pub async fn insert_category(
    conn: &mut Conn,
    inst: &Installation,
    id: &str,
    name: &str,
) -> CoreResult<()> {
    let now = db::now();
    sqlx::query(
        "INSERT INTO categories (id, name, branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(name)
    .bind(&inst.branch_id)
    .bind(&inst.device_id)
    .bind(&now)
    .bind(&now)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn rename_category(conn: &mut Conn, id: &str, name: &str) -> CoreResult<()> {
    sqlx::query(
        "UPDATE categories SET name = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(name)
    .bind(db::now())
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn delete_category(conn: &mut Conn, id: &str) -> CoreResult<()> {
    sqlx::query("UPDATE categories SET deleted_at = ?, updated_at = ? WHERE id = ?")
        .bind(db::now())
        .bind(db::now())
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}
