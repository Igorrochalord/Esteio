use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};

use crate::audit::{self, Entry};
use crate::db;
use crate::error::{CoreError, CoreResult, Validator};
use crate::modules::customers::service as customers;
use crate::modules::finance::service as finance;
use crate::modules::products::repository::{self as products_repo, NewMovement};
use crate::modules::products::service as products;
use crate::modules::users::repository as users_repo;
use crate::money;
use crate::permissions::{self, Action, Module};
use crate::validate;
use crate::Ctx;

use super::models::{Sale, SaleInput, SaleItem, SaleItemRow, SaleQuery, SaleRow, PAYMENT_METHODS};
use super::repository;

const NOT_FOUND: &str = "Venda não encontrada";

fn sees_profit(ctx: &Ctx) -> bool {
    permissions::is_allowed(ctx.user.role(), Module::Reports, Action::Read)
}

fn item_out(r: SaleItemRow) -> SaleItem {
    SaleItem {
        qty: money::from_milli(r.qty_milli),
        unit_price: money::from_cents(r.unit_price_cents),
        discount: money::from_cents(r.discount_cents),
        total: money::from_cents(r.total_cents),
        id: r.id,
        product_id: r.product_id,
        product_name: r.product_name,
    }
}

fn to_out(r: SaleRow, with_profit: bool, items: Option<Vec<SaleItem>>) -> Sale {
    Sale {
        subtotal: money::from_cents(r.subtotal_cents),
        discount: money::from_cents(r.discount_cents),
        total: money::from_cents(r.total_cents),
        cost_total: with_profit.then(|| money::from_cents(r.cost_total_cents)),
        profit: with_profit.then(|| money::from_cents(r.total_cents - r.cost_total_cents)),
        id: r.id,
        number: r.number,
        customer_id: r.customer_id,
        customer_name: r.customer_name,
        seller_id: r.seller_id,
        seller_name: r.seller_name,
        status: r.status,
        payment_method: r.payment_method,
        notes: r.notes,
        sold_at: r.sold_at,
        canceled_at: r.canceled_at,
        cancel_reason: r.cancel_reason,
        item_count: r.item_count,
        items,
    }
}

/// Aceita data (`AAAA-MM-DD`, vira meio-dia UTC) ou data-hora RFC 3339. Devolve no formato do banco.
pub(crate) fn parse_when(value: &str) -> Option<String> {
    let v = value.trim();
    let dt: DateTime<Utc> = if let Ok(d) = NaiveDate::parse_from_str(v, "%Y-%m-%d") {
        d.and_hms_opt(12, 0, 0)?.and_utc()
    } else {
        DateTime::parse_from_rfc3339(v).ok()?.with_timezone(&Utc)
    };
    Some(dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
}

struct Line {
    product_id: String,
    product_name: String,
    track_stock: bool,
    qty_milli: i64,
    unit_price_cents: i64,
    unit_cost_cents: i64,
    discount_cents: i64,
    total_cents: i64,
}

/// Cria a venda dentro de uma transação já aberta (usada também ao faturar uma OS). Devolve o id.
pub(crate) async fn create_in(tx: &mut db::Conn, ctx: &Ctx, data: SaleInput) -> CoreResult<String> {
    let mut v = Validator::default();
    v.check(
        PAYMENT_METHODS.contains(&data.payment_method.as_str()),
        "payment_method",
        "Forma de pagamento inválida",
    );
    v.check(
        !data.items.is_empty(),
        "items",
        "Adicione pelo menos um produto",
    );
    v.check(
        data.payment_method != "fiado" || data.customer_id.is_some(),
        "customer_id",
        "Venda fiado precisa de um cliente",
    );
    let can_manage = permissions::is_allowed(ctx.user.role(), Module::Sales, Action::Manage);
    let sold_at = match &data.sold_at {
        Some(when) => {
            let parsed = parse_when(when);
            v.check(parsed.is_some(), "sold_at", "Data inválida");
            v.check(
                can_manage,
                "sold_at",
                "Só quem gerencia vendas pode registrar com outra data",
            );
            parsed.unwrap_or_else(db::now)
        }
        None => db::now(),
    };
    let sale_discount = data.discount.and_then(money::to_cents).unwrap_or(0);
    v.check(sale_discount >= 0, "discount", "Desconto inválido");
    v.finish()?;

    // vendedor e cliente
    let seller_id = data
        .seller_id
        .clone()
        .unwrap_or_else(|| ctx.user.id.clone());
    let seller = users_repo::get(tx, &seller_id).await?;
    if !seller.is_some_and(|s| s.is_active) {
        return Err(CoreError::field(
            "seller_id",
            "Vendedor inválido ou inativo",
        ));
    }
    if let Some(cid) = &data.customer_id {
        if !customers::get(tx, cid).await?.is_some_and(|c| c.active) {
            return Err(CoreError::field(
                "customer_id",
                "Cliente não encontrado ou inativo",
            ));
        }
    }

    // itens: preço/custo do cadastro, totais em centavos
    let mut lines = Vec::with_capacity(data.items.len());
    let mut needed: HashMap<String, i64> = HashMap::new();
    for (i, item) in data.items.iter().enumerate() {
        let p = products::load(tx, &item.product_id).await?;
        if !p.active {
            return Err(CoreError::BusinessRule(format!(
                "O produto {} está inativo",
                p.name
            )));
        }
        let qty = money::to_milli(item.qty)
            .filter(|q| *q > 0)
            .ok_or_else(|| CoreError::field(&format!("items.{i}.qty"), "Quantidade inválida"))?;
        let unit_price = match item.unit_price {
            Some(price) => money::to_cents(price).filter(|c| *c >= 0).ok_or_else(|| {
                CoreError::field(&format!("items.{i}.unit_price"), "Preço inválido")
            })?,
            None => p.price_cents,
        };
        let gross = money::line_cents(qty, unit_price);
        let discount = item.discount.and_then(money::to_cents).unwrap_or(0);
        if discount < 0 || discount > gross {
            return Err(CoreError::field(
                &format!("items.{i}.discount"),
                "Desconto maior que o valor do item",
            ));
        }
        if p.track_stock {
            *needed.entry(p.id.clone()).or_default() += qty;
            let total_needed = needed[&p.id];
            if total_needed > p.stock_milli {
                return Err(CoreError::BusinessRule(format!(
                    "Estoque insuficiente de {}: há {} {}",
                    p.name,
                    money::from_milli(p.stock_milli),
                    p.unit
                )));
            }
        }
        lines.push(Line {
            product_id: p.id,
            product_name: p.name,
            track_stock: p.track_stock,
            qty_milli: qty,
            unit_price_cents: unit_price,
            unit_cost_cents: p.cost_cents,
            discount_cents: discount,
            total_cents: gross - discount,
        });
    }
    let subtotal: i64 = lines.iter().map(|l| l.total_cents).sum();
    if sale_discount > subtotal {
        return Err(CoreError::field(
            "discount",
            "Desconto maior que o total da venda",
        ));
    }
    let total = subtotal - sale_discount;
    let cost_total: i64 = lines
        .iter()
        .map(|l| money::line_cents(l.qty_milli, l.unit_cost_cents))
        .sum();

    let id = db::new_id();
    let number = repository::next_number(tx, &ctx.inst.branch_id).await?;
    let now = db::now();
    sqlx::query(
        "INSERT INTO sales (id, number, customer_id, seller_id, status, payment_method,
            subtotal_cents, discount_cents, total_cents, cost_total_cents, notes, sold_at,
            branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, 'concluida', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(number)
    .bind(&data.customer_id)
    .bind(&seller_id)
    .bind(&data.payment_method)
    .bind(subtotal)
    .bind(sale_discount)
    .bind(total)
    .bind(cost_total)
    .bind(validate::opt_text(data.notes))
    .bind(&sold_at)
    .bind(&ctx.inst.branch_id)
    .bind(&ctx.inst.device_id)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    for l in &lines {
        sqlx::query(
            "INSERT INTO sale_items (id, sale_id, product_id, product_name, qty_milli,
                unit_price_cents, unit_cost_cents, discount_cents, total_cents,
                branch_id, device_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(db::new_id())
        .bind(&id)
        .bind(&l.product_id)
        .bind(&l.product_name)
        .bind(l.qty_milli)
        .bind(l.unit_price_cents)
        .bind(l.unit_cost_cents)
        .bind(l.discount_cents)
        .bind(l.total_cents)
        .bind(&ctx.inst.branch_id)
        .bind(&ctx.inst.device_id)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        if l.track_stock {
            let note = format!("Venda nº {number}");
            let m = NewMovement {
                product_id: &l.product_id,
                kind: "venda",
                delta_milli: -l.qty_milli,
                unit_cost_cents: Some(l.unit_cost_cents),
                note: Some(&note),
                sale_id: Some(&id),
                user_id: Some(&ctx.user.id),
            };
            products_repo::apply_movement(tx, &ctx.inst, m).await?;
        }
    }

    // financeiro: à vista entra na conta padrão; fiado vira conta a receber
    let offset = chrono::Duration::minutes(i64::from(
        data.tz_offset_minutes.unwrap_or(-180).clamp(-840, 840),
    ));
    let local_date = DateTime::parse_from_rfc3339(&sold_at)
        .map(|d| (d.with_timezone(&Utc) + offset).date_naive())
        .unwrap_or_else(|_| (Utc::now() + offset).date_naive());
    finance::on_sale_created(
        tx,
        &ctx.inst,
        &id,
        number,
        &data.payment_method,
        total,
        data.customer_id.as_deref(),
        local_date,
    )
    .await?;

    let entry = Entry {
        actor_id: Some(&ctx.user.id),
        action: "sale.create",
        entity: "sale",
        entity_id: Some(&id),
        before: None,
        after: Some(serde_json::json!({
            "number": number,
            "total": money::from_cents(total),
            "payment_method": data.payment_method,
            "items": lines.len(),
        })),
    };
    audit::record(tx, &ctx.inst, entry).await?;

    Ok(id)
}

pub async fn create(ctx: &Ctx, data: SaleInput) -> CoreResult<Sale> {
    let mut tx = ctx.db.begin().await?;
    let id = create_in(&mut tx, ctx, data).await?;
    let row = repository::get(&mut tx, &id)
        .await?
        .ok_or_else(|| CoreError::Internal("venda recém-criada sumiu".into()))?;
    let items = repository::items(&mut tx, &id).await?;
    tx.commit().await?;
    Ok(to_out(
        row,
        sees_profit(ctx),
        Some(items.into_iter().map(item_out).collect()),
    ))
}

pub async fn get(ctx: &Ctx, id: &str) -> CoreResult<Sale> {
    let mut conn = ctx.db.acquire().await?;
    let row = repository::get(&mut conn, id)
        .await?
        .ok_or_else(|| CoreError::NotFound(NOT_FOUND.into()))?;
    let items = repository::items(&mut conn, id).await?;
    Ok(to_out(
        row,
        sees_profit(ctx),
        Some(items.into_iter().map(item_out).collect()),
    ))
}

pub async fn list(ctx: &Ctx, q: SaleQuery) -> CoreResult<Vec<Sale>> {
    let from = q.from.as_deref().and_then(parse_bound);
    let to = q.to.as_deref().and_then(parse_bound);
    let limit = q.limit.unwrap_or(1000).clamp(1, 20_000);
    let mut conn = ctx.db.acquire().await?;
    let rows = repository::list(
        &mut conn,
        from.as_deref(),
        to.as_deref(),
        q.status.as_deref().filter(|s| !s.is_empty()),
        q.customer_id.as_deref().filter(|s| !s.is_empty()),
        q.seller_id.as_deref().filter(|s| !s.is_empty()),
        limit,
    )
    .await?;
    let with_profit = sees_profit(ctx);
    Ok(rows
        .into_iter()
        .map(|r| to_out(r, with_profit, None))
        .collect())
}

/// Limite de período: `AAAA-MM-DD` vira o início do dia (UTC); data-hora é usada como veio.
pub(crate) fn parse_bound(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    if let Ok(d) = NaiveDate::parse_from_str(v, "%Y-%m-%d") {
        return Some(format!("{}T00:00:00.000Z", d.format("%Y-%m-%d")));
    }
    parse_when(v)
}

/// Cancela a venda e devolve o estoque. A venda continua no histórico, marcada como cancelada.
pub async fn cancel(ctx: &Ctx, id: &str, reason: &str) -> CoreResult<Sale> {
    let reason = reason.trim();
    if !validate::len_between(reason, 3, 500) {
        return Err(CoreError::field(
            "reason",
            "Informe o motivo do cancelamento",
        ));
    }
    let mut tx = ctx.db.begin().await?;
    let row = repository::get(&mut tx, id)
        .await?
        .ok_or_else(|| CoreError::NotFound(NOT_FOUND.into()))?;
    if row.status == "cancelada" {
        return Err(CoreError::BusinessRule(
            "Esta venda já está cancelada".into(),
        ));
    }
    let now = db::now();
    sqlx::query(
        "UPDATE sales SET status = 'cancelada', canceled_at = ?, canceled_by = ?, cancel_reason = ?,
            updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(&ctx.user.id)
    .bind(reason)
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    // estorna exatamente o que a venda baixou
    let sold: Vec<(String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT product_id, qty_milli, unit_cost_cents FROM stock_movements
         WHERE sale_id = ? AND kind = 'venda'",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    let note = format!("Cancelamento da venda nº {}", row.number);
    for (product_id, qty, cost) in sold {
        let m = NewMovement {
            product_id: &product_id,
            kind: "estorno_venda",
            delta_milli: -qty,
            unit_cost_cents: cost,
            note: Some(&note),
            sale_id: Some(id),
            user_id: Some(&ctx.user.id),
        };
        products_repo::apply_movement(&mut tx, &ctx.inst, m).await?;
    }

    finance::on_sale_canceled(&mut tx, id).await?;

    let entry = Entry {
        actor_id: Some(&ctx.user.id),
        action: "sale.cancel",
        entity: "sale",
        entity_id: Some(id),
        before: Some(serde_json::json!({ "status": "concluida" })),
        after: Some(
            serde_json::json!({ "status": "cancelada", "reason": reason, "number": row.number }),
        ),
    };
    audit::record(&mut tx, &ctx.inst, entry).await?;
    let updated = repository::get(&mut tx, id)
        .await?
        .ok_or_else(|| CoreError::NotFound(NOT_FOUND.into()))?;
    let items = repository::items(&mut tx, id).await?;
    tx.commit().await?;
    Ok(to_out(
        updated,
        sees_profit(ctx),
        Some(items.into_iter().map(item_out).collect()),
    ))
}
