//! Ordens de serviço. Fluxo: aberta → em andamento → aguardando → concluída → faturada.
//! Faturar gera uma venda na MESMA transação (peças baixam do estoque, valor vai ao financeiro).
// PLANO: ordens de serviço são recurso do Starter em diante (hoje liberado).

use crate::audit::{self, Entry};
use crate::db::{self, Conn};
use crate::error::{CoreError, CoreResult, Validator};
use crate::modules::finance::service::parse_date;
use crate::modules::sales::models::{SaleInput, SaleItemInput, PAYMENT_METHODS};
use crate::modules::sales::service as sales;
use crate::money::{self, from_cents, from_milli};
use crate::validate;
use crate::Ctx;

use super::models::*;

const NOT_FOUND: &str = "Ordem de serviço não encontrada";

#[derive(sqlx::FromRow)]
struct OsRow {
    id: String,
    number: i64,
    customer_id: String,
    customer_name: Option<String>,
    customer_phone: Option<String>,
    status: String,
    equipment: Option<String>,
    reported: String,
    diagnosis: Option<String>,
    technician_id: Option<String>,
    technician_name: Option<String>,
    scheduled_date: Option<String>,
    due_date: Option<String>,
    discount_cents: i64,
    subtotal_cents: i64,
    sale_id: Option<String>,
    sale_number: Option<i64>,
    finished_at: Option<String>,
    canceled_reason: Option<String>,
    created_at: String,
    updated_at: String,
    item_count: i64,
}

const SELECT: &str = "SELECT o.id, o.number, o.customer_id, c.name AS customer_name, c.phone AS customer_phone,
        o.status, o.equipment, o.reported, o.diagnosis, o.technician_id, u.name AS technician_name,
        o.scheduled_date, o.due_date, o.discount_cents,
        coalesce((SELECT sum((i.qty_milli * i.unit_price_cents + 500) / 1000 - i.discount_cents)
                  FROM service_order_items i WHERE i.order_id = o.id AND i.deleted_at IS NULL), 0) AS subtotal_cents,
        o.sale_id, s.number AS sale_number, o.finished_at, o.canceled_reason, o.created_at, o.updated_at,
        (SELECT count(*) FROM service_order_items i WHERE i.order_id = o.id AND i.deleted_at IS NULL) AS item_count
    FROM service_orders o
    LEFT JOIN customers c ON c.id = o.customer_id
    LEFT JOIN users u ON u.id = o.technician_id
    LEFT JOIN sales s ON s.id = o.sale_id
    WHERE o.deleted_at IS NULL";

fn out(r: OsRow, items: Option<Vec<OsItem>>, events: Option<Vec<OsEvent>>) -> ServiceOrder {
    ServiceOrder {
        subtotal: from_cents(r.subtotal_cents),
        discount: from_cents(r.discount_cents),
        total: from_cents((r.subtotal_cents - r.discount_cents).max(0)),
        id: r.id,
        number: r.number,
        customer_id: r.customer_id,
        customer_name: r.customer_name,
        customer_phone: r.customer_phone,
        status: r.status,
        equipment: r.equipment,
        reported: r.reported,
        diagnosis: r.diagnosis,
        technician_id: r.technician_id,
        technician_name: r.technician_name,
        scheduled_date: r.scheduled_date,
        due_date: r.due_date,
        sale_id: r.sale_id,
        sale_number: r.sale_number,
        finished_at: r.finished_at,
        canceled_reason: r.canceled_reason,
        created_at: r.created_at,
        updated_at: r.updated_at,
        item_count: r.item_count,
        items,
        events,
    }
}

async fn row(conn: &mut Conn, id: &str) -> CoreResult<OsRow> {
    let sql = format!("{SELECT} AND o.id = ?");
    sqlx::query_as(&sql)
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| CoreError::NotFound(NOT_FOUND.into()))
}

async fn items(conn: &mut Conn, id: &str) -> CoreResult<Vec<OsItem>> {
    let rows: Vec<(String, String, String, bool, i64, i64, i64)> = sqlx::query_as(
        "SELECT i.id, i.product_id, i.description, coalesce(p.track_stock = 0, 1), i.qty_milli,
                i.unit_price_cents, i.discount_cents
         FROM service_order_items i LEFT JOIN products p ON p.id = i.product_id
         WHERE i.order_id = ? AND i.deleted_at IS NULL ORDER BY i.created_at, i.id",
    )
    .bind(id)
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, product_id, description, is_service, qty, price, disc)| OsItem {
                total: from_cents(money::line_cents(qty, price) - disc),
                qty: from_milli(qty),
                unit_price: from_cents(price),
                discount: from_cents(disc),
                id,
                product_id,
                description,
                is_service,
            },
        )
        .collect())
}

type Opt = Option<String>;
type EventRow = (String, String, Opt, Opt, Opt, Opt, String);

async fn events(conn: &mut Conn, id: &str) -> CoreResult<Vec<OsEvent>> {
    let rows: Vec<EventRow> = sqlx::query_as(
        "SELECT e.id, e.kind, e.from_status, e.to_status, e.note, u.name, e.created_at
             FROM service_order_events e LEFT JOIN users u ON u.id = e.user_id
             WHERE e.order_id = ? ORDER BY e.created_at, e.id",
    )
    .bind(id)
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, kind, from_status, to_status, note, user_name, created_at)| OsEvent {
                id,
                kind,
                from_status,
                to_status,
                note,
                user_name,
                created_at,
            },
        )
        .collect())
}

async fn full(conn: &mut Conn, id: &str) -> CoreResult<ServiceOrder> {
    let r = row(conn, id).await?;
    let it = items(conn, id).await?;
    let ev = events(conn, id).await?;
    Ok(out(r, Some(it), Some(ev)))
}

async fn event(
    tx: &mut Conn,
    ctx: &Ctx,
    order_id: &str,
    kind: &str,
    from: Option<&str>,
    to: Option<&str>,
    note: Option<&str>,
) -> CoreResult<()> {
    let now = db::now();
    sqlx::query(
        "INSERT INTO service_order_events (id, order_id, user_id, kind, from_status, to_status, note,
            branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(db::new_id())
    .bind(order_id)
    .bind(&ctx.user.id)
    .bind(kind)
    .bind(from)
    .bind(to)
    .bind(note)
    .bind(&ctx.inst.branch_id)
    .bind(&ctx.inst.device_id)
    .bind(&now)
    .bind(&now)
    .execute(tx)
    .await?;
    Ok(())
}

fn editable(status: &str) -> CoreResult<()> {
    if matches!(status, "faturada" | "cancelada") {
        return Err(CoreError::BusinessRule(
            "OS faturada ou cancelada não pode ser alterada".into(),
        ));
    }
    Ok(())
}

fn opt_date(v: &mut Validator, field: &str, s: &Option<String>) -> Option<String> {
    let s = s.as_deref().map(str::trim).filter(|x| !x.is_empty())?;
    let d = parse_date(s);
    v.check(d.is_some(), field, "Data inválida");
    d.map(|d| d.format("%Y-%m-%d").to_string())
}

async fn check_technician(conn: &mut Conn, id: Option<&str>) -> CoreResult<()> {
    if let Some(id) = id {
        let ok: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM users WHERE id = ? AND is_active = 1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_one(conn)
        .await?;
        if ok == 0 {
            return Err(CoreError::field(
                "technician_id",
                "Técnico inválido ou inativo",
            ));
        }
    }
    Ok(())
}

/// Substitui os itens da OS (a tela edita a lista inteira).
async fn write_items(
    tx: &mut Conn,
    ctx: &Ctx,
    order_id: &str,
    list: &[OsItemInput],
) -> CoreResult<()> {
    let now = db::now();
    sqlx::query("UPDATE service_order_items SET deleted_at = ?, updated_at = ? WHERE order_id = ? AND deleted_at IS NULL")
        .bind(&now)
        .bind(&now)
        .bind(order_id)
        .execute(&mut *tx)
        .await?;
    for (i, it) in list.iter().enumerate() {
        let p: Option<(String, i64, bool)> = sqlx::query_as(
            "SELECT name, price_cents, active FROM products WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(&it.product_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (name, price, active) = p.ok_or_else(|| {
            CoreError::field(
                &format!("items.{i}.product_id"),
                "Produto ou serviço não encontrado",
            )
        })?;
        if !active {
            return Err(CoreError::BusinessRule(format!("{name} está inativo")));
        }
        let qty = money::to_milli(it.qty)
            .filter(|q| *q > 0)
            .ok_or_else(|| CoreError::field(&format!("items.{i}.qty"), "Quantidade inválida"))?;
        let unit = match it.unit_price {
            Some(p) => money::to_cents(p).filter(|c| *c >= 0).ok_or_else(|| {
                CoreError::field(&format!("items.{i}.unit_price"), "Preço inválido")
            })?,
            None => price,
        };
        let disc = it.discount.and_then(money::to_cents).unwrap_or(0);
        if disc < 0 || disc > money::line_cents(qty, unit) {
            return Err(CoreError::field(
                &format!("items.{i}.discount"),
                "Desconto maior que o item",
            ));
        }
        let desc = validate::opt_text(it.description.clone()).unwrap_or(name);
        sqlx::query(
            "INSERT INTO service_order_items (id, order_id, product_id, description, qty_milli, unit_price_cents,
                discount_cents, branch_id, device_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(db::new_id())
        .bind(order_id)
        .bind(&it.product_id)
        .bind(&desc)
        .bind(qty)
        .bind(unit)
        .bind(disc)
        .bind(&ctx.inst.branch_id)
        .bind(&ctx.inst.device_id)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

pub async fn list(ctx: &Ctx, q: OsQuery) -> CoreResult<Vec<ServiceOrder>> {
    let mut conn = ctx.db.acquire().await?;
    let sql = format!(
        "{SELECT} AND (?1 IS NULL OR o.status = ?1) AND (?2 IS NULL OR o.customer_id = ?2)
           AND (?3 IS NULL OR o.technician_id = ?3)
         ORDER BY o.number DESC LIMIT 5000"
    );
    let ne = |o: &Option<String>| o.as_deref().filter(|s| !s.is_empty()).map(str::to_owned);
    let rows: Vec<OsRow> = sqlx::query_as(&sql)
        .bind(ne(&q.status))
        .bind(ne(&q.customer_id))
        .bind(ne(&q.technician_id))
        .fetch_all(&mut *conn)
        .await?;
    Ok(rows.into_iter().map(|r| out(r, None, None)).collect())
}

pub async fn get(ctx: &Ctx, id: &str) -> CoreResult<ServiceOrder> {
    let mut conn = ctx.db.acquire().await?;
    full(&mut conn, id).await
}

pub async fn create(ctx: &Ctx, data: OsInput) -> CoreResult<ServiceOrder> {
    let mut v = Validator::default();
    v.check(
        validate::len_between(&data.reported, 2, 2000),
        "reported",
        "Descreva o problema ou o pedido do cliente",
    );
    let scheduled = opt_date(&mut v, "scheduled_date", &data.scheduled_date);
    let due = opt_date(&mut v, "due_date", &data.due_date);
    v.finish()?;
    let mut tx = ctx.db.begin().await?;
    let ok: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM customers WHERE id = ? AND active = 1 AND deleted_at IS NULL",
    )
    .bind(&data.customer_id)
    .fetch_one(&mut *tx)
    .await?;
    if ok == 0 {
        return Err(CoreError::field("customer_id", "Escolha um cliente ativo"));
    }
    check_technician(&mut tx, data.technician_id.as_deref()).await?;
    let max: Option<i64> =
        sqlx::query_scalar("SELECT max(number) FROM service_orders WHERE branch_id = ?")
            .bind(&ctx.inst.branch_id)
            .fetch_one(&mut *tx)
            .await?;
    let number = max.unwrap_or(0) + 1;
    let id = db::new_id();
    let now = db::now();
    sqlx::query(
        "INSERT INTO service_orders (id, number, customer_id, equipment, reported, technician_id, scheduled_date,
            due_date, branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(number)
    .bind(&data.customer_id)
    .bind(validate::opt_text(data.equipment))
    .bind(data.reported.trim())
    .bind(&data.technician_id)
    .bind(&scheduled)
    .bind(&due)
    .bind(&ctx.inst.branch_id)
    .bind(&ctx.inst.device_id)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    write_items(&mut tx, ctx, &id, &data.items).await?;
    event(&mut tx, ctx, &id, "criada", None, Some("aberta"), None).await?;
    let e = Entry {
        actor_id: Some(&ctx.user.id),
        action: "os.create",
        entity: "service_order",
        entity_id: Some(&id),
        before: None,
        after: Some(serde_json::json!({ "number": number })),
    };
    audit::record(&mut tx, &ctx.inst, e).await?;
    let os = full(&mut tx, &id).await?;
    tx.commit().await?;
    Ok(os)
}

pub async fn update(ctx: &Ctx, id: &str, data: OsPatch) -> CoreResult<ServiceOrder> {
    let mut v = Validator::default();
    if let Some(r) = &data.reported {
        v.check(
            validate::len_between(r, 2, 2000),
            "reported",
            "Descreva o problema ou o pedido do cliente",
        );
    }
    let scheduled = data
        .scheduled_date
        .as_ref()
        .map(|d| opt_date(&mut v, "scheduled_date", d));
    let due = data
        .due_date
        .as_ref()
        .map(|d| opt_date(&mut v, "due_date", d));
    let discount = data.discount.map(|d| {
        let c = money::to_cents(d);
        v.check(c.is_some_and(|c| c >= 0), "discount", "Desconto inválido");
        c.unwrap_or(0)
    });
    v.finish()?;
    let mut tx = ctx.db.begin().await?;
    let cur = row(&mut tx, id).await?;
    editable(&cur.status)?;
    if let Some(t) = &data.technician_id {
        check_technician(&mut tx, t.as_deref()).await?;
    }
    let pick = |o: &Option<Option<String>>, c: &Option<String>| match o {
        Some(v) => validate::opt_text(v.clone()),
        None => c.clone(),
    };
    sqlx::query(
        "UPDATE service_orders SET equipment = ?, reported = ?, diagnosis = ?, technician_id = ?,
            scheduled_date = ?, due_date = ?, discount_cents = ?, updated_at = ? WHERE id = ?",
    )
    .bind(pick(&data.equipment, &cur.equipment))
    .bind(
        data.reported
            .as_deref()
            .map(str::trim)
            .unwrap_or(&cur.reported),
    )
    .bind(pick(&data.diagnosis, &cur.diagnosis))
    .bind(match &data.technician_id {
        Some(t) => t.clone(),
        None => cur.technician_id.clone(),
    })
    .bind(scheduled.unwrap_or(cur.scheduled_date.clone()))
    .bind(due.unwrap_or(cur.due_date.clone()))
    .bind(discount.unwrap_or(cur.discount_cents))
    .bind(db::now())
    .bind(id)
    .execute(&mut *tx)
    .await?;
    let os = full(&mut tx, id).await?;
    tx.commit().await?;
    Ok(os)
}

pub async fn set_items(ctx: &Ctx, id: &str, list: Vec<OsItemInput>) -> CoreResult<ServiceOrder> {
    let mut tx = ctx.db.begin().await?;
    let cur = row(&mut tx, id).await?;
    editable(&cur.status)?;
    write_items(&mut tx, ctx, id, &list).await?;
    sqlx::query("UPDATE service_orders SET updated_at = ? WHERE id = ?")
        .bind(db::now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let os = full(&mut tx, id).await?;
    tx.commit().await?;
    Ok(os)
}

pub async fn change_status(
    ctx: &Ctx,
    id: &str,
    to: &str,
    note: Option<String>,
) -> CoreResult<ServiceOrder> {
    if !WORK_STATUSES.contains(&to) {
        return Err(CoreError::field(
            "status",
            "Situação inválida (para faturar ou cancelar use a ação própria)",
        ));
    }
    let mut tx = ctx.db.begin().await?;
    let cur = row(&mut tx, id).await?;
    editable(&cur.status)?;
    if cur.status == to {
        return full(&mut tx, id).await;
    }
    let now = db::now();
    sqlx::query(
        "UPDATE service_orders SET status = ?, finished_at = ?, updated_at = ? WHERE id = ?",
    )
    .bind(to)
    .bind((to == "concluida").then_some(now.clone()))
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    let note = validate::opt_text(note);
    event(
        &mut tx,
        ctx,
        id,
        "status",
        Some(&cur.status),
        Some(to),
        note.as_deref(),
    )
    .await?;
    let os = full(&mut tx, id).await?;
    tx.commit().await?;
    Ok(os)
}

pub async fn add_note(ctx: &Ctx, id: &str, note: &str) -> CoreResult<ServiceOrder> {
    if !validate::len_between(note, 1, 2000) {
        return Err(CoreError::field("note", "Escreva a anotação"));
    }
    let mut tx = ctx.db.begin().await?;
    row(&mut tx, id).await?;
    event(&mut tx, ctx, id, "nota", None, None, Some(note.trim())).await?;
    let os = full(&mut tx, id).await?;
    tx.commit().await?;
    Ok(os)
}

/// Fatura a OS concluída: gera a venda (estoque + financeiro) na mesma transação.
pub async fn invoice(ctx: &Ctx, id: &str, data: InvoiceInput) -> CoreResult<ServiceOrder> {
    if !PAYMENT_METHODS.contains(&data.payment_method.as_str()) {
        return Err(CoreError::field(
            "payment_method",
            "Forma de pagamento inválida",
        ));
    }
    let mut tx = ctx.db.begin().await?;
    let cur = row(&mut tx, id).await?;
    if cur.status != "concluida" {
        return Err(CoreError::BusinessRule(
            "Conclua a OS antes de faturar".into(),
        ));
    }
    let it = items(&mut tx, id).await?;
    if it.is_empty() {
        return Err(CoreError::BusinessRule(
            "Adicione serviços ou peças antes de faturar".into(),
        ));
    }
    let sale_input = SaleInput {
        customer_id: Some(cur.customer_id.clone()),
        seller_id: data.seller_id.or(cur.technician_id.clone()),
        payment_method: data.payment_method,
        discount: (cur.discount_cents > 0).then(|| from_cents(cur.discount_cents)),
        notes: Some(format!("OS nº {}", cur.number)),
        sold_at: None,
        tz_offset_minutes: data.tz_offset_minutes,
        pix_provider: None,
        items: it
            .iter()
            .map(|i| SaleItemInput {
                product_id: i.product_id.clone(),
                qty: i.qty,
                unit_price: Some(i.unit_price),
                discount: (i.discount > rust_decimal::Decimal::ZERO).then_some(i.discount),
            })
            .collect(),
    };
    let sale_id = sales::create_in(&mut tx, ctx, sale_input).await?;
    let now = db::now();
    sqlx::query(
        "UPDATE service_orders SET status = 'faturada', sale_id = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&sale_id)
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    event(
        &mut tx,
        ctx,
        id,
        "faturada",
        Some("concluida"),
        Some("faturada"),
        None,
    )
    .await?;
    let e = Entry {
        actor_id: Some(&ctx.user.id),
        action: "os.invoice",
        entity: "service_order",
        entity_id: Some(id),
        before: None,
        after: Some(serde_json::json!({ "number": cur.number, "sale_id": sale_id })),
    };
    audit::record(&mut tx, &ctx.inst, e).await?;
    let os = full(&mut tx, id).await?;
    tx.commit().await?;
    Ok(os)
}

pub async fn cancel(ctx: &Ctx, id: &str, reason: &str) -> CoreResult<ServiceOrder> {
    if !validate::len_between(reason, 3, 500) {
        return Err(CoreError::field("reason", "Informe o motivo"));
    }
    let mut tx = ctx.db.begin().await?;
    let cur = row(&mut tx, id).await?;
    if cur.status == "faturada" {
        return Err(CoreError::BusinessRule(
            "OS faturada: cancele a venda gerada em Vendas".into(),
        ));
    }
    if cur.status == "cancelada" {
        return Err(CoreError::BusinessRule("Esta OS já está cancelada".into()));
    }
    sqlx::query("UPDATE service_orders SET status = 'cancelada', canceled_reason = ?, updated_at = ? WHERE id = ?")
        .bind(reason.trim())
        .bind(db::now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    event(
        &mut tx,
        ctx,
        id,
        "status",
        Some(&cur.status),
        Some("cancelada"),
        Some(reason.trim()),
    )
    .await?;
    let e = Entry {
        actor_id: Some(&ctx.user.id),
        action: "os.cancel",
        entity: "service_order",
        entity_id: Some(id),
        before: None,
        after: Some(serde_json::json!({ "number": cur.number, "reason": reason.trim() })),
    };
    audit::record(&mut tx, &ctx.inst, e).await?;
    let os = full(&mut tx, id).await?;
    tx.commit().await?;
    Ok(os)
}
