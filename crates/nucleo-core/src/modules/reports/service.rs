//! Relatórios gerenciais. Somente leitura; somas em inteiros (centavos/milésimos) no SQLite
//! e conversão para Decimal só na saída. Vendas canceladas ficam fora (exceto nos contadores
//! de cancelamento).

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;

use crate::error::{CoreError, CoreResult};
use crate::modules::sales::service::parse_bound;
use crate::money::{from_cents, from_milli};
use crate::Ctx;

use super::models::*;

/// Período resolvido: limites em UTC (formato do banco) e modificador de fuso do SQLite.
struct Range {
    from: Option<String>,
    to: Option<String>,
    tz: String,
    tz_minutes: i32,
}

fn range(q: &ReportQuery) -> CoreResult<Range> {
    if !(-840..=840).contains(&q.tz_offset_minutes) {
        return Err(CoreError::field(
            "tz_offset_minutes",
            "Fuso horário inválido",
        ));
    }
    let from = q.from.as_deref().and_then(parse_bound);
    let to = q.to.as_deref().and_then(parse_bound);
    if let (Some(f), Some(t)) = (&from, &to) {
        if f >= t {
            return Err(CoreError::field(
                "to",
                "A data final precisa ser depois da inicial",
            ));
        }
    }
    Ok(Range {
        from,
        to,
        tz: format!("{:+} minutes", q.tz_offset_minutes),
        tz_minutes: q.tz_offset_minutes,
    })
}

/// Percentual com 1 casa (0 quando o todo é zero).
fn pct(part: i64, whole: i64) -> Decimal {
    if whole == 0 {
        return Decimal::ZERO;
    }
    (Decimal::from(part) * Decimal::ONE_HUNDRED / Decimal::from(whole)).round_dp(1)
}

fn avg_cents(total: i64, count: i64) -> Decimal {
    if count == 0 {
        Decimal::ZERO
    } else {
        from_cents((total + count / 2) / count)
    }
}

/// Formato SQLite do período (mesmo resultado que `chrono::format` em `fill_periods`).
fn period_format(group: &str) -> CoreResult<&'static str> {
    match group {
        "day" => Ok("%Y-%m-%d"),
        "week" => Ok("%Y-W%W"),
        "month" => Ok("%Y-%m"),
        _ => Err(CoreError::field(
            "group",
            "Agrupamento inválido (day, week ou month)",
        )),
    }
}

fn to_utc(v: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(v)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// Todas as chaves do período (inclusive as sem venda), em hora local.
fn fill_periods(r: &Range, fmt: &str) -> Vec<String> {
    let (Some(from), Some(to)) = (
        r.from.as_deref().and_then(to_utc),
        r.to.as_deref().and_then(to_utc),
    ) else {
        return vec![];
    };
    let offset = Duration::minutes(i64::from(r.tz_minutes));
    let first: NaiveDate = (from + offset).date_naive();
    let last: NaiveDate = (to + offset - Duration::milliseconds(1)).date_naive();
    if (last - first).num_days() > 3700 {
        return vec![]; // período enorme: não preenche lacunas
    }
    let mut keys = Vec::new();
    let mut day = first;
    while day <= last {
        let key = day.format(fmt).to_string();
        if keys.last() != Some(&key) {
            keys.push(key);
        }
        day += Duration::days(1);
    }
    keys
}

const RANGE: &str = "(?1 IS NULL OR s.sold_at >= ?1) AND (?2 IS NULL OR s.sold_at < ?2)";

pub async fn sales_summary(ctx: &Ctx, q: ReportQuery) -> CoreResult<SalesSummary> {
    let r = range(&q)?;
    let fmt = period_format(q.group.as_deref().unwrap_or("day"))?;
    let mut conn = ctx.db.acquire().await?;

    let (count, gross, discount, total, cost): (i64, i64, i64, i64, i64) =
        sqlx::query_as(&format!(
            "SELECT count(*), coalesce(sum(subtotal_cents), 0), coalesce(sum(discount_cents), 0),
                coalesce(sum(total_cents), 0), coalesce(sum(cost_total_cents), 0)
         FROM sales s WHERE s.status = 'concluida' AND {RANGE}"
        ))
        .bind(&r.from)
        .bind(&r.to)
        .fetch_one(&mut *conn)
        .await?;
    let items_qty: i64 = sqlx::query_scalar(&format!(
        "SELECT coalesce(sum(i.qty_milli), 0) FROM sale_items i JOIN sales s ON s.id = i.sale_id
         WHERE s.status = 'concluida' AND {RANGE}"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .fetch_one(&mut *conn)
    .await?;
    let (canceled_count, canceled_total): (i64, i64) = sqlx::query_as(&format!(
        "SELECT count(*), coalesce(sum(total_cents), 0) FROM sales s
         WHERE s.status = 'cancelada' AND {RANGE}"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .fetch_one(&mut *conn)
    .await?;

    let rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(&format!(
        "SELECT strftime(?3, s.sold_at, ?4) AS period, count(*), sum(total_cents), sum(cost_total_cents)
         FROM sales s WHERE s.status = 'concluida' AND {RANGE}
         GROUP BY period ORDER BY period"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .bind(fmt)
    .bind(&r.tz)
    .fetch_all(&mut *conn)
    .await?;
    let mut by_period: BTreeMap<String, (i64, i64, i64)> = fill_periods(&r, fmt)
        .into_iter()
        .map(|k| (k, (0, 0, 0)))
        .collect();
    for (period, c, t, k) in rows {
        by_period.insert(period, (c, t, k));
    }
    let series = by_period
        .into_iter()
        .map(|(period, (c, t, k))| PeriodRow {
            period,
            count: c,
            total: from_cents(t),
            cost: from_cents(k),
            profit: from_cents(t - k),
        })
        .collect();

    let pay: Vec<(String, i64, i64)> = sqlx::query_as(&format!(
        "SELECT payment_method, count(*), sum(total_cents) FROM sales s
         WHERE s.status = 'concluida' AND {RANGE}
         GROUP BY payment_method ORDER BY sum(total_cents) DESC"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .fetch_all(&mut *conn)
    .await?;
    let by_payment = pay
        .into_iter()
        .map(|(method, c, t)| PaymentRow {
            method,
            count: c,
            total: from_cents(t),
            share_pct: pct(t, total),
        })
        .collect();

    let hours: Vec<(String, i64, i64, i64)> = sqlx::query_as(&format!(
        "SELECT strftime('%H', s.sold_at, ?3) AS h, count(*), sum(total_cents), sum(cost_total_cents)
         FROM sales s WHERE s.status = 'concluida' AND {RANGE} GROUP BY h ORDER BY h"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .bind(&r.tz)
    .fetch_all(&mut *conn)
    .await?;
    let mut by_hour_map: BTreeMap<String, (i64, i64, i64)> =
        (0..24).map(|h| (format!("{h:02}"), (0, 0, 0))).collect();
    for (h, c, t, k) in hours {
        by_hour_map.insert(h, (c, t, k));
    }
    let by_hour = by_hour_map
        .into_iter()
        .map(|(period, (c, t, k))| PeriodRow {
            period,
            count: c,
            total: from_cents(t),
            cost: from_cents(k),
            profit: from_cents(t - k),
        })
        .collect();

    Ok(SalesSummary {
        totals: SalesTotals {
            count,
            gross: from_cents(gross),
            discount: from_cents(discount),
            total: from_cents(total),
            cost: from_cents(cost),
            profit: from_cents(total - cost),
            margin_pct: pct(total - cost, total),
            avg_ticket: avg_cents(total, count),
            items_qty: from_milli(items_qty),
            canceled_count,
            canceled_total: from_cents(canceled_total),
        },
        series,
        by_payment,
        by_hour,
    })
}

/// Custo de uma linha em SQL, igual a `money::line_cents` (arredondamento meio-para-cima).
const LINE_COST: &str = "(i.qty_milli * i.unit_cost_cents + 500) / 1000";

pub async fn products(ctx: &Ctx, q: ReportQuery) -> CoreResult<ProductsReport> {
    let r = range(&q)?;
    let mut conn = ctx.db.acquire().await?;
    type Row = (
        String,
        Option<String>,
        String,
        Option<String>,
        String,
        i64,
        i64,
        i64,
    );
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT i.product_id, p.code, max(i.product_name), c.name, coalesce(p.unit, 'un'),
                sum(i.qty_milli), sum(i.total_cents), sum({LINE_COST})
         FROM sale_items i
         JOIN sales s ON s.id = i.sale_id
         LEFT JOIN products p ON p.id = i.product_id
         LEFT JOIN categories c ON c.id = p.category_id AND c.deleted_at IS NULL
         WHERE s.status = 'concluida' AND {RANGE}
         GROUP BY i.product_id ORDER BY sum(i.total_cents) DESC, max(i.product_name)"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .fetch_all(&mut *conn)
    .await?;

    let grand: i64 = rows.iter().map(|r| r.6).sum();
    let mut cumulative = 0i64;
    let mut categories: BTreeMap<String, (i64, i64, i64)> = BTreeMap::new();
    let mut products = Vec::with_capacity(rows.len());
    for (product_id, code, name, category, unit, qty, revenue, cost) in rows {
        // classe pelo acumulado ANTES do item: o item que cruza 80% ainda é A
        let class = match pct(cumulative, grand) {
            p if p < Decimal::from(80) => "A",
            p if p < Decimal::from(95) => "B",
            _ => "C",
        };
        cumulative += revenue;
        let cat = categories
            .entry(category.clone().unwrap_or_else(|| "Sem categoria".into()))
            .or_default();
        cat.0 += qty;
        cat.1 += revenue;
        cat.2 += cost;
        products.push(ProductRank {
            product_id,
            code,
            name,
            category,
            unit,
            qty: from_milli(qty),
            revenue: from_cents(revenue),
            cost: from_cents(cost),
            profit: from_cents(revenue - cost),
            margin_pct: pct(revenue - cost, revenue),
            share_pct: pct(revenue, grand),
            cumulative_pct: pct(cumulative, grand),
            abc: class.into(),
        });
    }
    if let Some(limit) = q.limit {
        products.truncate(limit.clamp(1, 10_000) as usize);
    }
    let mut categories: Vec<CategoryRow> = categories
        .into_iter()
        .map(|(category, (qty, revenue, cost))| CategoryRow {
            category,
            qty: from_milli(qty),
            revenue: from_cents(revenue),
            cost: from_cents(cost),
            profit: from_cents(revenue - cost),
            margin_pct: pct(revenue - cost, revenue),
        })
        .collect();
    categories.sort_by(|a, b| b.revenue.cmp(&a.revenue));
    Ok(ProductsReport {
        products,
        categories,
    })
}

pub async fn stock(ctx: &Ctx, q: ReportQuery) -> CoreResult<StockReport> {
    let r = range(&q)?;
    let mut conn = ctx.db.acquire().await?;
    type Row = (
        String,
        String,
        String,
        Option<String>,
        String,
        i64,
        i64,
        i64,
        i64,
        i64,
    );
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT p.id, p.code, p.name, c.name, p.unit, p.stock_milli, p.min_stock_milli,
                p.cost_cents, p.price_cents,
                coalesce((SELECT sum(i.qty_milli) FROM sale_items i JOIN sales s ON s.id = i.sale_id
                          WHERE i.product_id = p.id AND s.status = 'concluida' AND {RANGE}), 0)
         FROM products p
         LEFT JOIN categories c ON c.id = p.category_id AND c.deleted_at IS NULL
         WHERE p.deleted_at IS NULL AND p.active = 1 AND p.track_stock = 1
         ORDER BY p.name COLLATE NOCASE"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .fetch_all(&mut *conn)
    .await?;

    let days = match (
        r.from.as_deref().and_then(to_utc),
        r.to.as_deref().and_then(to_utc),
    ) {
        (Some(f), Some(t)) => Some(Decimal::from(
            ((t - f).num_hours() + 23).div_euclid(24).max(1),
        )),
        _ => None,
    };
    let (mut cost_total, mut price_total, mut below, mut out) = (0i64, 0i64, 0i64, 0i64);
    let rows = rows
        .into_iter()
        .map(
            |(id, code, name, category, unit, stock, min, cost, price, sold)| {
                let cost_value = crate::money::line_cents(stock.max(0), cost);
                let price_value = crate::money::line_cents(stock.max(0), price);
                cost_total += cost_value;
                price_total += price_value;
                let below_min = stock < min;
                below += i64::from(below_min);
                out += i64::from(stock <= 0);
                let days_of_cover = match days {
                    Some(d) if sold > 0 => {
                        Some((Decimal::from(stock.max(0)) * d / Decimal::from(sold)).round_dp(0))
                    }
                    _ => None,
                };
                StockRow {
                    product_id: id,
                    code,
                    name,
                    category,
                    unit,
                    stock: from_milli(stock),
                    min_stock: from_milli(min),
                    below_min,
                    cost_value: from_cents(cost_value),
                    price_value: from_cents(price_value),
                    sold_qty: from_milli(sold),
                    days_of_cover,
                }
            },
        )
        .collect();
    Ok(StockReport {
        rows,
        total_cost_value: from_cents(cost_total),
        total_price_value: from_cents(price_total),
        below_min_count: below,
        out_of_stock_count: out,
    })
}

pub async fn customers(ctx: &Ctx, q: ReportQuery) -> CoreResult<CustomersReport> {
    let r = range(&q)?;
    let mut conn = ctx.db.acquire().await?;
    let base = format!(
        "FROM sales s JOIN customers c ON c.id = s.customer_id
         WHERE s.status = 'concluida' AND {RANGE}"
    );

    let states: Vec<(String, i64, i64, i64)> = sqlx::query_as(&format!(
        "SELECT c.state, count(DISTINCT c.id), count(*), sum(s.total_cents) {base}
           AND c.state IS NOT NULL GROUP BY c.state ORDER BY sum(s.total_cents) DESC"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .fetch_all(&mut *conn)
    .await?;
    let cities: Vec<(String, Option<String>, i64, i64, i64)> = sqlx::query_as(&format!(
        "SELECT c.city, c.state, count(DISTINCT c.id), count(*), sum(s.total_cents) {base}
           AND c.city IS NOT NULL GROUP BY lower(c.city), c.state
           ORDER BY sum(s.total_cents) DESC LIMIT 30"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .fetch_all(&mut *conn)
    .await?;
    let limit = q.limit.unwrap_or(30).clamp(1, 1000);
    type Top = (
        String,
        String,
        Option<String>,
        Option<String>,
        i64,
        i64,
        String,
    );
    let top: Vec<Top> = sqlx::query_as(&format!(
        "SELECT c.id, c.name, c.city, c.state, count(*), sum(s.total_cents), max(s.sold_at) {base}
           GROUP BY c.id ORDER BY sum(s.total_cents) DESC LIMIT ?3"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .bind(limit)
    .fetch_all(&mut *conn)
    .await?;
    let (anonymous_count, anonymous_total): (i64, i64) = sqlx::query_as(&format!(
        "SELECT count(*), coalesce(sum(total_cents), 0) FROM sales s
         WHERE s.status = 'concluida' AND s.customer_id IS NULL AND {RANGE}"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .fetch_one(&mut *conn)
    .await?;
    let without_state: i64 = sqlx::query_scalar(&format!(
        "SELECT count(DISTINCT c.id) {base} AND c.state IS NULL"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .fetch_one(&mut *conn)
    .await?;

    Ok(CustomersReport {
        by_state: states
            .into_iter()
            .map(|(state, customers, count, total)| StateRow {
                state,
                customers,
                count,
                total: from_cents(total),
            })
            .collect(),
        by_city: cities
            .into_iter()
            .map(|(city, state, customers, count, total)| CityRow {
                city,
                state,
                customers,
                count,
                total: from_cents(total),
            })
            .collect(),
        top_customers: top
            .into_iter()
            .map(|(id, name, city, state, count, total, last)| CustomerRank {
                customer_id: id,
                name,
                city,
                state,
                count,
                total: from_cents(total),
                avg_ticket: avg_cents(total, count),
                last_purchase: last,
            })
            .collect(),
        anonymous_count,
        anonymous_total: from_cents(anonymous_total),
        without_state,
    })
}

pub async fn sellers(ctx: &Ctx, q: ReportQuery) -> CoreResult<Vec<SellerRow>> {
    let r = range(&q)?;
    let mut conn = ctx.db.acquire().await?;
    type Row = (String, String, i64, i64, i64, i64, i64, i64);
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT u.id, u.name,
                count(*) FILTER (WHERE s.status = 'concluida'),
                coalesce(sum(s.total_cents) FILTER (WHERE s.status = 'concluida'), 0),
                coalesce(sum(s.cost_total_cents) FILTER (WHERE s.status = 'concluida'), 0),
                count(DISTINCT s.customer_id) FILTER (WHERE s.status = 'concluida'),
                coalesce((SELECT sum(i.qty_milli) FROM sale_items i JOIN sales s2 ON s2.id = i.sale_id
                          WHERE s2.seller_id = u.id AND s2.status = 'concluida'
                            AND (?1 IS NULL OR s2.sold_at >= ?1) AND (?2 IS NULL OR s2.sold_at < ?2)), 0),
                count(*) FILTER (WHERE s.status = 'cancelada')
         FROM sales s JOIN users u ON u.id = s.seller_id
         WHERE {RANGE}
         GROUP BY u.id ORDER BY 4 DESC"
    ))
    .bind(&r.from)
    .bind(&r.to)
    .fetch_all(&mut *conn)
    .await?;
    let grand: i64 = rows.iter().map(|r| r.3).sum();
    Ok(rows
        .into_iter()
        .map(
            |(id, name, count, total, cost, served, qty, canceled)| SellerRow {
                seller_id: id,
                name,
                count,
                total: from_cents(total),
                avg_ticket: avg_cents(total, count),
                profit: from_cents(total - cost),
                margin_pct: pct(total - cost, total),
                customers_served: served,
                items_qty: from_milli(qty),
                canceled_count: canceled,
                share_pct: pct(total, grand),
            },
        )
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periods_cover_local_days() {
        // 01/10 00:00 a 04/10 00:00 em Brasília (UTC-3)
        let r = Range {
            from: Some("2026-10-01T03:00:00.000Z".into()),
            to: Some("2026-10-04T03:00:00.000Z".into()),
            tz: "-180 minutes".into(),
            tz_minutes: -180,
        };
        assert_eq!(
            fill_periods(&r, "%Y-%m-%d"),
            ["2026-10-01", "2026-10-02", "2026-10-03"]
        );
        assert_eq!(fill_periods(&r, "%Y-%m"), ["2026-10"]);
    }
}
