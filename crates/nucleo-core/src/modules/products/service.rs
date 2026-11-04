use rust_decimal::Decimal;

use crate::audit::{self, Entry};
use crate::db::{self, Conn};
use crate::error::{CoreError, CoreResult, Validator};
use crate::money;
use crate::permissions::{self, Action, Module};
use crate::validate;
use crate::Ctx;

use super::models::{
    Category, Movement, MovementQuery, Product, ProductInput, ProductPatch, ProductRow, StockInput,
    MANUAL_KINDS, UNITS,
};
use super::repository::{self, NewMovement};

const NOT_FOUND: &str = "Produto não encontrado";

/// Quem não pode editar produtos não vê o custo.
fn sees_cost(ctx: &Ctx) -> bool {
    permissions::is_allowed(ctx.user.role(), Module::Products, Action::Write)
}

pub(crate) fn to_out(row: ProductRow, with_cost: bool) -> Product {
    Product {
        price: money::from_cents(row.price_cents),
        cost: with_cost.then(|| money::from_cents(row.cost_cents)),
        stock: money::from_milli(row.stock_milli),
        min_stock: money::from_milli(row.min_stock_milli),
        below_min: row.track_stock && row.active && row.stock_milli < row.min_stock_milli,
        id: row.id,
        code: row.code,
        barcode: row.barcode,
        name: row.name,
        category_id: row.category_id,
        category_name: row.category_name,
        unit: row.unit,
        track_stock: row.track_stock,
        active: row.active,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

pub async fn list(ctx: &Ctx) -> CoreResult<Vec<Product>> {
    let mut conn = ctx.db.acquire().await?;
    let with_cost = sees_cost(ctx);
    Ok(repository::list(&mut conn)
        .await?
        .into_iter()
        .map(|r| to_out(r, with_cost))
        .collect())
}

pub(crate) async fn load(conn: &mut Conn, id: &str) -> CoreResult<ProductRow> {
    repository::get(conn, id)
        .await?
        .ok_or_else(|| CoreError::NotFound(NOT_FOUND.into()))
}

/// Converte e valida valores monetários/quantidades vindos da tela.
fn cents(v: &mut Validator, field: &str, value: Decimal) -> i64 {
    let c = money::to_cents(value);
    v.check(
        c.is_some_and(|c| c >= 0),
        field,
        "Informe um valor maior ou igual a zero",
    );
    c.unwrap_or(0)
}

fn milli(v: &mut Validator, field: &str, value: Decimal, allow_negative: bool) -> i64 {
    let m = money::to_milli(value);
    v.check(
        m.is_some_and(|m| allow_negative || m >= 0),
        field,
        "Quantidade inválida",
    );
    m.unwrap_or(0)
}

async fn check_category(v: &mut Validator, conn: &mut Conn, id: Option<&str>) -> CoreResult<()> {
    if let Some(id) = id {
        let ok = repository::category_exists(conn, id).await?;
        v.check(ok, "category_id", "Categoria não encontrada");
    }
    Ok(())
}

async fn check_code_free(conn: &mut Conn, code: &str, except_id: Option<&str>) -> CoreResult<()> {
    if let Some(other) = repository::find_by_code(conn, code).await? {
        if Some(other.id.as_str()) != except_id {
            return Err(CoreError::Conflict(format!(
                "Já existe um produto com o código {code}"
            )));
        }
    }
    Ok(())
}

pub async fn create(ctx: &Ctx, data: ProductInput) -> CoreResult<Product> {
    let mut v = Validator::default();
    v.check(
        validate::len_between(&data.name, 1, 200),
        "name",
        "Informe o nome do produto",
    );
    let unit = data.unit.as_deref().unwrap_or("un").trim().to_lowercase();
    v.check(UNITS.contains(&unit.as_str()), "unit", "Unidade inválida");
    let price = cents(&mut v, "price", data.price);
    let cost = data.cost.map_or(0, |c| cents(&mut v, "cost", c));
    let min_stock = data
        .min_stock
        .map_or(0, |m| milli(&mut v, "min_stock", m, false));
    let initial = data
        .initial_stock
        .map(|q| milli(&mut v, "initial_stock", q, false))
        .filter(|&q| q > 0);
    let code = data.code.trim().to_owned();
    v.check(code.chars().count() <= 40, "code", "Código muito longo");

    let mut tx = ctx.db.begin().await?;
    check_category(&mut v, &mut tx, data.category_id.as_deref()).await?;
    v.finish()?;
    let code = if code.is_empty() {
        repository::next_code(&mut tx).await?
    } else {
        check_code_free(&mut tx, &code, None).await?;
        code
    };

    let now = db::now();
    let row = ProductRow {
        id: db::new_id(),
        code,
        barcode: validate::opt_text(data.barcode),
        name: data.name.trim().to_owned(),
        category_id: data.category_id,
        category_name: None,
        unit,
        price_cents: price,
        cost_cents: cost,
        track_stock: data.track_stock.unwrap_or(true),
        stock_milli: 0,
        min_stock_milli: min_stock,
        active: true,
        created_at: now.clone(),
        updated_at: now,
    };
    repository::insert(&mut tx, &ctx.inst, &row).await?;
    if let (Some(qty), true) = (initial, row.track_stock) {
        let m = NewMovement {
            product_id: &row.id,
            kind: "entrada",
            delta_milli: qty,
            unit_cost_cents: Some(cost),
            note: Some("Estoque inicial"),
            sale_id: None,
            user_id: Some(&ctx.user.id),
        };
        repository::apply_movement(&mut tx, &ctx.inst, m).await?;
    }
    let saved = load(&mut tx, &row.id).await?;
    audit_product(&mut tx, ctx, "product.create", None, &saved).await?;
    tx.commit().await?;
    Ok(to_out(saved, sees_cost(ctx)))
}

async fn audit_product(
    tx: &mut Conn,
    ctx: &Ctx,
    action: &str,
    before: Option<&ProductRow>,
    after: &ProductRow,
) -> CoreResult<()> {
    let snap = |p: &ProductRow| audit::snapshot(&to_out(p.clone(), true));
    let entry = Entry {
        actor_id: Some(&ctx.user.id),
        action,
        entity: "product",
        entity_id: Some(&after.id),
        before: before.map(snap),
        after: Some(snap(after)),
    };
    if before.is_some() {
        audit::record_changes(tx, &ctx.inst, entry).await
    } else {
        audit::record(tx, &ctx.inst, entry).await
    }
}

/// Edição parcial: cada célula da planilha manda só o campo alterado.
pub async fn update(ctx: &Ctx, id: &str, data: ProductPatch) -> CoreResult<Product> {
    let mut v = Validator::default();
    let mut tx = ctx.db.begin().await?;
    let before = load(&mut tx, id).await?;
    let mut p = before.clone();

    if let Some(name) = data.name {
        v.check(
            validate::len_between(&name, 1, 200),
            "name",
            "Informe o nome do produto",
        );
        p.name = name.trim().to_owned();
    }
    if let Some(code) = data.code {
        let code = code.trim().to_owned();
        v.check(
            !code.is_empty() && code.chars().count() <= 40,
            "code",
            "Informe o código (até 40 caracteres)",
        );
        if !code.is_empty() {
            check_code_free(&mut tx, &code, Some(id)).await?;
        }
        p.code = code;
    }
    if let Some(barcode) = data.barcode {
        p.barcode = validate::opt_text(barcode);
    }
    if let Some(category) = data.category_id {
        check_category(&mut v, &mut tx, category.as_deref()).await?;
        p.category_id = category;
    }
    if let Some(unit) = data.unit {
        let unit = unit.trim().to_lowercase();
        v.check(UNITS.contains(&unit.as_str()), "unit", "Unidade inválida");
        p.unit = unit;
    }
    if let Some(price) = data.price {
        p.price_cents = cents(&mut v, "price", price);
    }
    if let Some(cost) = data.cost {
        p.cost_cents = cents(&mut v, "cost", cost);
    }
    if let Some(track) = data.track_stock {
        p.track_stock = track;
    }
    if let Some(min) = data.min_stock {
        p.min_stock_milli = milli(&mut v, "min_stock", min, false);
    }
    if let Some(active) = data.active {
        p.active = active;
    }
    v.finish()?;

    p.updated_at = db::now();
    repository::update(&mut tx, &p).await?;
    let saved = load(&mut tx, id).await?;
    audit_product(&mut tx, ctx, "product.update", Some(&before), &saved).await?;
    tx.commit().await?;
    Ok(to_out(saved, sees_cost(ctx)))
}

/// Entrada, saída ou ajuste (contagem) manual de estoque.
pub async fn move_stock(ctx: &Ctx, data: StockInput) -> CoreResult<Product> {
    let mut v = Validator::default();
    v.check(
        MANUAL_KINDS.contains(&data.kind.as_str()),
        "kind",
        "Tipo de movimentação inválido",
    );
    let qty = milli(&mut v, "qty", data.qty, false);
    let unit_cost = data.unit_cost.map(|c| cents(&mut v, "unit_cost", c));
    v.finish()?;

    let mut tx = ctx.db.begin().await?;
    let mut product = load(&mut tx, &data.product_id).await?;
    if !product.track_stock {
        return Err(CoreError::BusinessRule(
            "Este produto não controla estoque".into(),
        ));
    }
    let delta = match data.kind.as_str() {
        "entrada" => qty,
        "saida" => -qty,
        _ => qty - product.stock_milli, // ajuste: qty é o saldo contado
    };
    if delta == 0 && data.kind != "ajuste" {
        return Err(CoreError::field(
            "qty",
            "Informe uma quantidade maior que zero",
        ));
    }
    if data.kind == "saida" && product.stock_milli + delta < 0 {
        return Err(CoreError::BusinessRule(format!(
            "Estoque insuficiente: há {} {}",
            money::from_milli(product.stock_milli),
            product.unit
        )));
    }
    if delta != 0 {
        let note = validate::opt_text(data.note);
        let m = NewMovement {
            product_id: &product.id,
            kind: &data.kind,
            delta_milli: delta,
            unit_cost_cents: unit_cost,
            note: note.as_deref(),
            sale_id: None,
            user_id: Some(&ctx.user.id),
        };
        repository::apply_movement(&mut tx, &ctx.inst, m).await?;
    }
    // entrada com custo informado atualiza o custo do produto (último custo)
    if let (Some(c), "entrada") = (unit_cost, data.kind.as_str()) {
        if c != product.cost_cents {
            let before = load(&mut tx, &product.id).await?;
            product = before.clone();
            product.cost_cents = c;
            product.updated_at = db::now();
            repository::update(&mut tx, &product).await?;
            let saved = load(&mut tx, &product.id).await?;
            audit_product(&mut tx, ctx, "product.update", Some(&before), &saved).await?;
        }
    }
    let saved = load(&mut tx, &data.product_id).await?;
    tx.commit().await?;
    Ok(to_out(saved, sees_cost(ctx)))
}

pub async fn movements(ctx: &Ctx, q: MovementQuery) -> CoreResult<Vec<Movement>> {
    let mut conn = ctx.db.acquire().await?;
    let limit = q.limit.unwrap_or(500).clamp(1, 5000);
    let with_cost = sees_cost(ctx);
    let rows = repository::movements(
        &mut conn,
        q.product_id.as_deref(),
        q.from.as_deref(),
        q.to.as_deref(),
        limit,
    )
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Movement {
            qty: money::from_milli(r.qty_milli),
            balance: money::from_milli(r.balance_milli),
            unit_cost: r
                .unit_cost_cents
                .filter(|_| with_cost)
                .map(money::from_cents),
            id: r.id,
            product_id: r.product_id,
            product_name: r.product_name,
            kind: r.kind,
            note: r.note,
            sale_id: r.sale_id,
            user_name: r.user_name,
            created_at: r.created_at,
        })
        .collect())
}

pub async fn categories(ctx: &Ctx) -> CoreResult<Vec<Category>> {
    let mut conn = ctx.db.acquire().await?;
    repository::categories(&mut conn).await
}

fn category_name(name: &str) -> CoreResult<String> {
    if !validate::len_between(name, 1, 80) {
        return Err(CoreError::field("name", "Informe o nome da categoria"));
    }
    Ok(name.trim().to_owned())
}

pub async fn create_category(ctx: &Ctx, name: &str) -> CoreResult<Category> {
    let name = category_name(name)?;
    let mut tx = ctx.db.begin().await?;
    let cat = upsert_category(&mut tx, ctx, &name).await?;
    tx.commit().await?;
    Ok(cat)
}

/// Devolve a categoria com esse nome, criando se não existir (usado também na importação).
pub(crate) async fn upsert_category(tx: &mut Conn, ctx: &Ctx, name: &str) -> CoreResult<Category> {
    if let Some(found) = repository::category_by_name(tx, name).await? {
        return Ok(found);
    }
    let id = db::new_id();
    repository::insert_category(tx, &ctx.inst, &id, name).await?;
    let entry = Entry {
        actor_id: Some(&ctx.user.id),
        action: "category.create",
        entity: "category",
        entity_id: Some(&id),
        before: None,
        after: Some(serde_json::json!({ "name": name })),
    };
    audit::record(tx, &ctx.inst, entry).await?;
    Ok(Category {
        id,
        name: name.to_owned(),
        product_count: 0,
    })
}

pub async fn rename_category(ctx: &Ctx, id: &str, name: &str) -> CoreResult<()> {
    let name = category_name(name)?;
    let mut tx = ctx.db.begin().await?;
    if !repository::category_exists(&mut tx, id).await? {
        return Err(CoreError::NotFound("Categoria não encontrada".into()));
    }
    if let Some(other) = repository::category_by_name(&mut tx, &name).await? {
        if other.id != id {
            return Err(CoreError::Conflict(
                "Já existe uma categoria com esse nome".into(),
            ));
        }
    }
    repository::rename_category(&mut tx, id, &name).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn delete_category(ctx: &Ctx, id: &str) -> CoreResult<()> {
    let mut tx = ctx.db.begin().await?;
    let cats = repository::categories(&mut tx).await?;
    let cat = cats
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| CoreError::NotFound("Categoria não encontrada".into()))?;
    if cat.product_count > 0 {
        return Err(CoreError::BusinessRule(format!(
            "A categoria tem {} produto(s): mova-os antes de excluir",
            cat.product_count
        )));
    }
    repository::delete_category(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}
