use crate::db::Conn;
use crate::error::CoreResult;

use super::models::{SaleItemRow, SaleRow};

const SELECT: &str = "SELECT s.id, s.number, s.customer_id, c.name AS customer_name, s.seller_id,
        u.name AS seller_name, s.status, s.payment_method, s.subtotal_cents, s.discount_cents,
        s.total_cents, s.cost_total_cents, s.notes, s.sold_at, s.canceled_at, s.cancel_reason,
        (SELECT count(*) FROM sale_items i WHERE i.sale_id = s.id) AS item_count
    FROM sales s
    LEFT JOIN customers c ON c.id = s.customer_id
    LEFT JOIN users u ON u.id = s.seller_id
    WHERE s.deleted_at IS NULL";

pub async fn get(conn: &mut Conn, id: &str) -> CoreResult<Option<SaleRow>> {
    let sql = format!("{SELECT} AND s.id = ?");
    Ok(sqlx::query_as(&sql).bind(id).fetch_optional(conn).await?)
}

#[allow(clippy::too_many_arguments)]
pub async fn list(
    conn: &mut Conn,
    from: Option<&str>,
    to: Option<&str>,
    status: Option<&str>,
    customer_id: Option<&str>,
    seller_id: Option<&str>,
    limit: i64,
) -> CoreResult<Vec<SaleRow>> {
    let sql = format!(
        "{SELECT} AND (?1 IS NULL OR s.sold_at >= ?1) AND (?2 IS NULL OR s.sold_at < ?2)
           AND (?3 IS NULL OR s.status = ?3) AND (?4 IS NULL OR s.customer_id = ?4)
           AND (?5 IS NULL OR s.seller_id = ?5)
         ORDER BY s.sold_at DESC, s.number DESC LIMIT ?6"
    );
    Ok(sqlx::query_as(&sql)
        .bind(from)
        .bind(to)
        .bind(status)
        .bind(customer_id)
        .bind(seller_id)
        .bind(limit)
        .fetch_all(conn)
        .await?)
}

pub async fn items(conn: &mut Conn, sale_id: &str) -> CoreResult<Vec<SaleItemRow>> {
    Ok(sqlx::query_as(
        "SELECT id, product_id, product_name, qty_milli, unit_price_cents, unit_cost_cents,
                discount_cents, total_cents
         FROM sale_items WHERE sale_id = ? ORDER BY created_at, id",
    )
    .bind(sale_id)
    .fetch_all(conn)
    .await?)
}

pub async fn next_number(conn: &mut Conn, branch_id: &str) -> CoreResult<i64> {
    let max: Option<i64> = sqlx::query_scalar("SELECT max(number) FROM sales WHERE branch_id = ?")
        .bind(branch_id)
        .fetch_one(conn)
        .await?;
    Ok(max.unwrap_or(0) + 1)
}
