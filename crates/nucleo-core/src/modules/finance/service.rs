//! Financeiro: contas bancárias, categorias, títulos a pagar/receber, movimentações,
//! transferências e a ponte com as vendas (à vista = entrada na conta padrão; fiado = a receber).

use chrono::{Months, NaiveDate};
use rust_decimal::Decimal;

use crate::audit::{self, Entry as AuditEntry};
use crate::db::{self, Conn, Installation};
use crate::error::{CoreError, CoreResult, Validator};
use crate::money::{from_cents, to_cents};
use crate::plans;
use crate::validate;
use crate::Ctx;

use super::models::*;

// ------------------------------------------------------------------ utilidades

/// Data AAAA-MM-DD válida.
pub(crate) fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
}

fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

fn fmt(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn positive_cents(v: &mut Validator, field: &str, value: Decimal) -> i64 {
    let c = to_cents(value);
    v.check(
        c.is_some_and(|c| c > 0),
        field,
        "Informe um valor maior que zero",
    );
    c.unwrap_or(0)
}

async fn audit(
    tx: &mut Conn,
    ctx: &Ctx,
    action: &str,
    entity: &str,
    id: &str,
    after: serde_json::Value,
) -> CoreResult<()> {
    let e = AuditEntry {
        actor_id: Some(&ctx.user.id),
        action,
        entity,
        entity_id: Some(id),
        before: None,
        after: Some(after),
    };
    audit::record(tx, &ctx.inst, e).await
}

// ------------------------------------------------------------------ categorias

/// (nome, tipo, grupo da DRE, chave do sistema)
const DEFAULT_CATEGORIES: [(&str, &str, &str, Option<&str>); 13] = [
    ("Vendas", "receita", "receita_vendas", Some("vendas")),
    ("Serviços", "receita", "receita_vendas", Some("servicos")),
    ("Outras receitas", "receita", "outras_receitas", None),
    (
        "Rendimentos e juros recebidos",
        "receita",
        "receitas_financeiras",
        None,
    ),
    ("Compras e fornecedores", "despesa", "custos", None),
    ("Aluguel", "despesa", "despesas_operacionais", None),
    (
        "Energia, água e internet",
        "despesa",
        "despesas_operacionais",
        None,
    ),
    ("Marketing", "despesa", "despesas_operacionais", None),
    ("Outras despesas", "despesa", "despesas_operacionais", None),
    ("Salários e encargos", "despesa", "despesas_pessoal", None),
    ("Impostos e taxas", "despesa", "impostos", None),
    (
        "Tarifas bancárias e juros pagos",
        "despesa",
        "despesas_financeiras",
        None,
    ),
    (
        "Investimentos e equipamentos",
        "despesa",
        "investimentos",
        None,
    ),
];

/// Cria as categorias padrão na primeira vez (idempotente).
pub(crate) async fn ensure_default_categories(
    conn: &mut Conn,
    inst: &Installation,
) -> CoreResult<()> {
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM fin_categories")
        .fetch_one(&mut *conn)
        .await?;
    if n > 0 {
        return Ok(());
    }
    let now = db::now();
    for (name, kind, group, key) in DEFAULT_CATEGORIES {
        sqlx::query(
            "INSERT INTO fin_categories (id, name, kind, dre_group, system_key, branch_id, device_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(db::new_id())
        .bind(name)
        .bind(kind)
        .bind(group)
        .bind(key)
        .bind(&inst.branch_id)
        .bind(&inst.device_id)
        .bind(&now)
        .bind(&now)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

async fn system_category(conn: &mut Conn, key: &str) -> CoreResult<String> {
    sqlx::query_scalar("SELECT id FROM fin_categories WHERE system_key = ? AND deleted_at IS NULL")
        .bind(key)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| CoreError::Internal(format!("categoria do sistema {key} ausente")))
}

pub async fn categories(ctx: &Ctx) -> CoreResult<Vec<Category>> {
    let mut tx = ctx.db.begin().await?;
    ensure_default_categories(&mut tx, &ctx.inst).await?;
    let rows = sqlx::query_as(
        "SELECT id, name, kind, dre_group, system_key, active FROM fin_categories
         WHERE deleted_at IS NULL ORDER BY kind DESC, name COLLATE NOCASE",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows)
}

async fn category_kind(conn: &mut Conn, id: &str) -> CoreResult<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT kind FROM fin_categories WHERE id = ? AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

pub async fn create_category(ctx: &Ctx, data: CategoryInput) -> CoreResult<Category> {
    let mut v = Validator::default();
    v.check(
        validate::len_between(&data.name, 1, 80),
        "name",
        "Informe o nome",
    );
    v.check(
        matches!(data.kind.as_str(), "receita" | "despesa"),
        "kind",
        "Tipo inválido",
    );
    v.check(
        DRE_GROUPS.contains(&data.dre_group.as_str()),
        "dre_group",
        "Grupo da DRE inválido",
    );
    v.finish()?;
    let mut tx = ctx.db.begin().await?;
    ensure_default_categories(&mut tx, &ctx.inst).await?;
    let id = db::new_id();
    let now = db::now();
    let r = sqlx::query(
        "INSERT INTO fin_categories (id, name, kind, dre_group, branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(data.name.trim())
    .bind(&data.kind)
    .bind(&data.dre_group)
    .bind(&ctx.inst.branch_id)
    .bind(&ctx.inst.device_id)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await;
    if r.is_err() {
        return Err(CoreError::Conflict(
            "Já existe uma categoria com esse nome".into(),
        ));
    }
    audit(
        &mut tx,
        ctx,
        "fin_category.create",
        "fin_category",
        &id,
        serde_json::json!({"name": data.name.trim()}),
    )
    .await?;
    tx.commit().await?;
    Ok(Category {
        id,
        name: data.name.trim().to_owned(),
        kind: data.kind,
        dre_group: data.dre_group,
        system_key: None,
        active: true,
    })
}

pub async fn update_category(ctx: &Ctx, id: &str, data: CategoryPatch) -> CoreResult<()> {
    let mut v = Validator::default();
    if let Some(n) = &data.name {
        v.check(validate::len_between(n, 1, 80), "name", "Informe o nome");
    }
    if let Some(g) = &data.dre_group {
        v.check(
            DRE_GROUPS.contains(&g.as_str()),
            "dre_group",
            "Grupo da DRE inválido",
        );
    }
    v.finish()?;
    let mut tx = ctx.db.begin().await?;
    let cur: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT name, system_key FROM fin_categories WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let (_, key) = cur.ok_or_else(|| CoreError::NotFound("Categoria não encontrada".into()))?;
    if key.is_some() && (data.name.is_some() || data.active == Some(false)) {
        return Err(CoreError::BusinessRule(
            "Categoria do sistema: só o grupo da DRE pode mudar".into(),
        ));
    }
    sqlx::query(
        "UPDATE fin_categories SET name = coalesce(?, name), dre_group = coalesce(?, dre_group),
            active = coalesce(?, active), updated_at = ? WHERE id = ?",
    )
    .bind(data.name.as_deref().map(str::trim))
    .bind(&data.dre_group)
    .bind(data.active)
    .bind(db::now())
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| CoreError::Conflict("Já existe uma categoria com esse nome".into()))?;
    audit(
        &mut tx,
        ctx,
        "fin_category.update",
        "fin_category",
        id,
        serde_json::json!({"name": data.name, "dre_group": data.dre_group, "active": data.active}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

// ------------------------------------------------------------------ contas

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: String,
    name: String,
    kind: String,
    bank_name: Option<String>,
    agency: Option<String>,
    account_number: Option<String>,
    initial_balance_cents: i64,
    opening_date: String,
    is_default: bool,
    active: bool,
    balance_cents: i64,
    unreconciled: i64,
}

const ACCOUNT_SELECT: &str = "SELECT a.id, a.name, a.kind, a.bank_name, a.agency, a.account_number,
        a.initial_balance_cents, a.opening_date, a.is_default, a.active,
        a.initial_balance_cents + coalesce((SELECT sum(t.amount_cents) FROM bank_transactions t
            WHERE t.bank_account_id = a.id AND t.deleted_at IS NULL), 0) AS balance_cents,
        (SELECT count(*) FROM bank_transactions t WHERE t.bank_account_id = a.id
            AND t.deleted_at IS NULL AND t.reconciled = 0) AS unreconciled
    FROM bank_accounts a WHERE a.deleted_at IS NULL";

fn account_out(r: AccountRow) -> Account {
    Account {
        initial_balance: from_cents(r.initial_balance_cents),
        balance: from_cents(r.balance_cents),
        id: r.id,
        name: r.name,
        kind: r.kind,
        bank_name: r.bank_name,
        agency: r.agency,
        account_number: r.account_number,
        opening_date: r.opening_date,
        is_default: r.is_default,
        active: r.active,
        unreconciled: r.unreconciled,
    }
}

pub async fn accounts(ctx: &Ctx) -> CoreResult<Vec<Account>> {
    let mut conn = ctx.db.acquire().await?;
    let sql = format!("{ACCOUNT_SELECT} ORDER BY a.is_default DESC, a.name COLLATE NOCASE");
    let rows: Vec<AccountRow> = sqlx::query_as(&sql).fetch_all(&mut *conn).await?;
    Ok(rows.into_iter().map(account_out).collect())
}

async fn load_account(conn: &mut Conn, id: &str) -> CoreResult<AccountRow> {
    let sql = format!("{ACCOUNT_SELECT} AND a.id = ?");
    sqlx::query_as(&sql)
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| CoreError::NotFound("Conta não encontrada".into()))
}

async fn clear_default(conn: &mut Conn, except: &str) -> CoreResult<()> {
    sqlx::query(
        "UPDATE bank_accounts SET is_default = 0, updated_at = ? WHERE is_default = 1 AND id <> ?",
    )
    .bind(db::now())
    .bind(except)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn create_account(ctx: &Ctx, data: AccountInput) -> CoreResult<Account> {
    let mut v = Validator::default();
    v.check(
        validate::len_between(&data.name, 1, 80),
        "name",
        "Informe o nome da conta",
    );
    v.check(
        ACCOUNT_KINDS.contains(&data.kind.as_str()),
        "kind",
        "Tipo de conta inválido",
    );
    let initial = data.initial_balance.map(to_cents).unwrap_or(Some(0));
    v.check(
        initial.is_some(),
        "initial_balance",
        "Saldo inicial inválido",
    );
    let opening = match &data.opening_date {
        Some(d) => parse_date(d),
        None => Some(today()),
    };
    v.check(opening.is_some(), "opening_date", "Data inválida");
    v.finish()?;

    let mut tx = ctx.db.begin().await?;
    let active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bank_accounts WHERE deleted_at IS NULL AND active = 1",
    )
    .fetch_one(&mut *tx)
    .await?;
    // PLANO: Gratuito permite até 3 contas bancárias (hoje liberado)
    plans::ensure_within(plans::Feature::BankAccounts, active as u32)?;
    let id = db::new_id();
    let now = db::now();
    let is_default = data.is_default.unwrap_or(active == 0);
    if is_default {
        clear_default(&mut tx, &id).await?;
    }
    sqlx::query(
        "INSERT INTO bank_accounts (id, name, kind, bank_name, agency, account_number, initial_balance_cents,
            opening_date, is_default, branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(data.name.trim())
    .bind(&data.kind)
    .bind(validate::opt_text(data.bank_name))
    .bind(validate::opt_text(data.agency))
    .bind(validate::opt_text(data.account_number))
    .bind(initial.unwrap_or(0))
    .bind(fmt(opening.unwrap_or_else(today)))
    .bind(is_default)
    .bind(&ctx.inst.branch_id)
    .bind(&ctx.inst.device_id)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await
    .map_err(|_| CoreError::Conflict("Já existe uma conta com esse nome".into()))?;
    audit(
        &mut tx,
        ctx,
        "bank_account.create",
        "bank_account",
        &id,
        serde_json::json!({"name": data.name.trim(), "kind": data.kind}),
    )
    .await?;
    let row = load_account(&mut tx, &id).await?;
    tx.commit().await?;
    Ok(account_out(row))
}

pub async fn update_account(ctx: &Ctx, id: &str, data: AccountPatch) -> CoreResult<Account> {
    let mut v = Validator::default();
    if let Some(n) = &data.name {
        v.check(
            validate::len_between(n, 1, 80),
            "name",
            "Informe o nome da conta",
        );
    }
    if let Some(k) = &data.kind {
        v.check(
            ACCOUNT_KINDS.contains(&k.as_str()),
            "kind",
            "Tipo de conta inválido",
        );
    }
    let initial = data.initial_balance.map(to_cents);
    v.check(
        !matches!(initial, Some(None)),
        "initial_balance",
        "Saldo inicial inválido",
    );
    v.finish()?;
    let mut tx = ctx.db.begin().await?;
    let cur = load_account(&mut tx, id).await?;
    if data.active == Some(false) && cur.is_default {
        return Err(CoreError::BusinessRule(
            "Escolha outra conta padrão antes de desativar esta".into(),
        ));
    }
    if data.is_default == Some(true) {
        clear_default(&mut tx, id).await?;
    }
    let opt = |o: &Option<Option<String>>, curv: &Option<String>| match o {
        Some(v) => v
            .as_ref()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty()),
        None => curv.clone(),
    };
    sqlx::query(
        "UPDATE bank_accounts SET name = ?, kind = ?, bank_name = ?, agency = ?, account_number = ?,
            initial_balance_cents = ?, is_default = ?, active = ?, updated_at = ? WHERE id = ?",
    )
    .bind(data.name.as_deref().map(str::trim).unwrap_or(&cur.name))
    .bind(data.kind.as_deref().unwrap_or(&cur.kind))
    .bind(opt(&data.bank_name, &cur.bank_name))
    .bind(opt(&data.agency, &cur.agency))
    .bind(opt(&data.account_number, &cur.account_number))
    .bind(initial.flatten().unwrap_or(cur.initial_balance_cents))
    .bind(data.is_default.unwrap_or(cur.is_default))
    .bind(data.active.unwrap_or(cur.active))
    .bind(db::now())
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| CoreError::Conflict("Já existe uma conta com esse nome".into()))?;
    audit(&mut tx, ctx, "bank_account.update", "bank_account", id, serde_json::json!({"name": data.name, "active": data.active, "is_default": data.is_default})).await?;
    let row = load_account(&mut tx, id).await?;
    tx.commit().await?;
    Ok(account_out(row))
}

/// Conta que recebe as vendas à vista: a padrão, senão a primeira ativa, senão cria "Caixa".
pub(crate) async fn default_account(conn: &mut Conn, inst: &Installation) -> CoreResult<String> {
    let found: Option<String> = sqlx::query_scalar(
        "SELECT id FROM bank_accounts WHERE deleted_at IS NULL AND active = 1
         ORDER BY is_default DESC, created_at LIMIT 1",
    )
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(id) = found {
        return Ok(id);
    }
    let id = db::new_id();
    let now = db::now();
    sqlx::query(
        "INSERT INTO bank_accounts (id, name, kind, opening_date, is_default, branch_id, device_id, created_at, updated_at)
         VALUES (?, 'Caixa', 'caixa', ?, 1, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(fmt(today()))
    .bind(&inst.branch_id)
    .bind(&inst.device_id)
    .bind(&now)
    .bind(&now)
    .execute(conn)
    .await?;
    Ok(id)
}

async fn active_account(conn: &mut Conn, id: &str) -> CoreResult<()> {
    let ok: Option<bool> =
        sqlx::query_scalar("SELECT active FROM bank_accounts WHERE id = ? AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(conn)
            .await?;
    match ok {
        Some(true) => Ok(()),
        _ => Err(CoreError::field(
            "bank_account_id",
            "Conta inválida ou inativa",
        )),
    }
}

// ------------------------------------------------------------------ movimentações

pub(crate) struct NewTx<'a> {
    pub account_id: &'a str,
    pub date: &'a str,
    pub amount_cents: i64,
    pub description: &'a str,
    pub category_id: Option<&'a str>,
    pub entry_id: Option<&'a str>,
    pub sale_id: Option<&'a str>,
    pub transfer_group: Option<&'a str>,
    pub fitid: Option<&'a str>,
    pub reconciled: bool,
}

pub(crate) async fn insert_tx(
    conn: &mut Conn,
    inst: &Installation,
    t: NewTx<'_>,
) -> CoreResult<String> {
    let id = db::new_id();
    let now = db::now();
    sqlx::query(
        "INSERT INTO bank_transactions (id, bank_account_id, date, amount_cents, description, category_id, entry_id,
            sale_id, transfer_group, ofx_fitid, reconciled, branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(t.account_id)
    .bind(t.date)
    .bind(t.amount_cents)
    .bind(t.description)
    .bind(t.category_id)
    .bind(t.entry_id)
    .bind(t.sale_id)
    .bind(t.transfer_group)
    .bind(t.fitid)
    .bind(t.reconciled)
    .bind(&inst.branch_id)
    .bind(&inst.device_id)
    .bind(&now)
    .bind(&now)
    .execute(conn)
    .await?;
    Ok(id)
}

#[derive(sqlx::FromRow)]
struct TxRow {
    id: String,
    bank_account_id: String,
    bank_account_name: String,
    date: String,
    amount_cents: i64,
    balance_after_cents: i64,
    description: String,
    category_id: Option<String>,
    category_name: Option<String>,
    entry_id: Option<String>,
    sale_id: Option<String>,
    transfer_group: Option<String>,
    reconciled: bool,
}

pub async fn transactions(ctx: &Ctx, q: TxQuery) -> CoreResult<Vec<Transaction>> {
    let mut conn = ctx.db.acquire().await?;
    // saldo após cada lançamento: saldo inicial + soma acumulada (por conta, em ordem de data)
    let rows: Vec<TxRow> = sqlx::query_as(
        "SELECT * FROM (
            SELECT t.id, t.bank_account_id, a.name AS bank_account_name, t.date, t.amount_cents,
                   a.initial_balance_cents + sum(t.amount_cents) OVER (
                       PARTITION BY t.bank_account_id ORDER BY t.date, t.created_at, t.id
                       ROWS UNBOUNDED PRECEDING) AS balance_after_cents,
                   t.description, t.category_id, c.name AS category_name, t.entry_id, t.sale_id,
                   t.transfer_group, t.reconciled
            FROM bank_transactions t
            JOIN bank_accounts a ON a.id = t.bank_account_id
            LEFT JOIN fin_categories c ON c.id = t.category_id
            WHERE t.deleted_at IS NULL AND (?1 IS NULL OR t.bank_account_id = ?1))
         WHERE (?2 IS NULL OR date >= ?2) AND (?3 IS NULL OR date <= ?3)
         ORDER BY date DESC, id DESC
         LIMIT 20000",
    )
    .bind(q.bank_account_id.as_deref().filter(|s| !s.is_empty()))
    .bind(q.from.as_deref().filter(|s| !s.is_empty()))
    .bind(q.to.as_deref().filter(|s| !s.is_empty()))
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Transaction {
            origin: if r.entry_id.is_some() {
                "titulo"
            } else if r.sale_id.is_some() {
                "venda"
            } else if r.transfer_group.is_some() {
                "transferencia"
            } else {
                "manual"
            }
            .into(),
            amount: from_cents(r.amount_cents),
            balance_after: from_cents(r.balance_after_cents),
            id: r.id,
            bank_account_id: r.bank_account_id,
            bank_account_name: r.bank_account_name,
            date: r.date,
            description: r.description,
            category_id: r.category_id,
            category_name: r.category_name,
            entry_id: r.entry_id,
            sale_id: r.sale_id,
            transfer_group: r.transfer_group,
            reconciled: r.reconciled,
        })
        .collect())
}

/// Lançamento avulso (entrada ou saída) numa conta.
pub async fn create_transaction(ctx: &Ctx, data: TxInput) -> CoreResult<()> {
    let mut v = Validator::default();
    let date = parse_date(&data.date);
    v.check(date.is_some(), "date", "Data inválida");
    v.check(
        matches!(data.kind.as_str(), "entrada" | "saida"),
        "kind",
        "Tipo inválido",
    );
    let cents = positive_cents(&mut v, "amount", data.amount);
    v.check(
        validate::len_between(&data.description, 1, 200),
        "description",
        "Informe a descrição",
    );
    v.finish()?;
    let mut tx = ctx.db.begin().await?;
    active_account(&mut tx, &data.bank_account_id).await?;
    if let Some(c) = &data.category_id {
        let kind = category_kind(&mut tx, c)
            .await?
            .ok_or_else(|| CoreError::field("category_id", "Categoria inválida"))?;
        let expected = if data.kind == "entrada" {
            "receita"
        } else {
            "despesa"
        };
        if kind != expected {
            return Err(CoreError::field(
                "category_id",
                "A categoria não combina com o tipo (receita/despesa)",
            ));
        }
    }
    let amount = if data.kind == "entrada" {
        cents
    } else {
        -cents
    };
    let date = fmt(date.unwrap_or_else(today));
    let id = insert_tx(
        &mut tx,
        &ctx.inst,
        NewTx {
            account_id: &data.bank_account_id,
            date: &date,
            amount_cents: amount,
            description: data.description.trim(),
            category_id: data.category_id.as_deref(),
            entry_id: None,
            sale_id: None,
            transfer_group: None,
            fitid: None,
            reconciled: false,
        },
    )
    .await?;
    audit(
        &mut tx,
        ctx,
        "bank_tx.create",
        "bank_transaction",
        &id,
        serde_json::json!({"amount": from_cents(amount), "description": data.description.trim()}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn transfer(ctx: &Ctx, data: TransferInput) -> CoreResult<()> {
    let mut v = Validator::default();
    let date = parse_date(&data.date);
    v.check(date.is_some(), "date", "Data inválida");
    let cents = positive_cents(&mut v, "amount", data.amount);
    v.check(
        data.from_account_id != data.to_account_id,
        "to_account_id",
        "Escolha contas diferentes",
    );
    v.finish()?;
    let mut tx = ctx.db.begin().await?;
    active_account(&mut tx, &data.from_account_id).await?;
    active_account(&mut tx, &data.to_account_id).await?;
    let group = db::new_id();
    let date = fmt(date.unwrap_or_else(today));
    let desc =
        validate::opt_text(data.description).unwrap_or_else(|| "Transferência entre contas".into());
    for (acc, amount) in [
        (&data.from_account_id, -cents),
        (&data.to_account_id, cents),
    ] {
        insert_tx(
            &mut tx,
            &ctx.inst,
            NewTx {
                account_id: acc,
                date: &date,
                amount_cents: amount,
                description: &desc,
                category_id: None,
                entry_id: None,
                sale_id: None,
                transfer_group: Some(&group),
                fitid: None,
                reconciled: false,
            },
        )
        .await?;
    }
    audit(
        &mut tx,
        ctx,
        "bank_tx.transfer",
        "bank_transaction",
        &group,
        serde_json::json!({"amount": from_cents(cents)}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Exclui lançamento avulso ou transferência (as duas pontas). Baixas e vendas se desfazem na origem.
pub async fn delete_transaction(ctx: &Ctx, id: &str) -> CoreResult<()> {
    let mut tx = ctx.db.begin().await?;
    let row: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT entry_id, sale_id, transfer_group FROM bank_transactions WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let (entry, sale, group) =
        row.ok_or_else(|| CoreError::NotFound("Lançamento não encontrado".into()))?;
    if entry.is_some() {
        return Err(CoreError::BusinessRule(
            "Este lançamento é a baixa de um título: estorne a baixa no título".into(),
        ));
    }
    if sale.is_some() {
        return Err(CoreError::BusinessRule(
            "Este lançamento é de uma venda: cancele a venda".into(),
        ));
    }
    let now = db::now();
    sqlx::query(
        // parâmetros todos numerados: misturar `?` e `?N` desloca as posições no SQLite
        "UPDATE bank_transactions SET deleted_at = ?1, updated_at = ?1
         WHERE deleted_at IS NULL AND (id = ?2 OR (?3 IS NOT NULL AND transfer_group = ?3))",
    )
    .bind(&now)
    .bind(id)
    .bind(&group)
    .execute(&mut *tx)
    .await?;
    audit(
        &mut tx,
        ctx,
        "bank_tx.delete",
        "bank_transaction",
        id,
        serde_json::json!({}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

// ------------------------------------------------------------------ títulos

#[derive(sqlx::FromRow)]
struct EntryRow {
    id: String,
    kind: String,
    description: String,
    category_id: String,
    category_name: Option<String>,
    customer_id: Option<String>,
    customer_name: Option<String>,
    counterparty: Option<String>,
    amount_cents: i64,
    issue_date: String,
    due_date: String,
    status: String,
    paid_date: Option<String>,
    paid_amount_cents: Option<i64>,
    bank_account_id: Option<String>,
    bank_account_name: Option<String>,
    payment_method: Option<String>,
    document: Option<String>,
    installment: Option<i64>,
    installments: Option<i64>,
    group_id: Option<String>,
    sale_id: Option<String>,
    sale_number: Option<i64>,
    notes: Option<String>,
}

const ENTRY_SELECT: &str = "SELECT e.id, e.kind, e.description, e.category_id, c.name AS category_name,
        e.customer_id, cu.name AS customer_name, e.counterparty, e.amount_cents, e.issue_date, e.due_date,
        e.status, e.paid_date, e.paid_amount_cents, e.bank_account_id, a.name AS bank_account_name,
        e.payment_method, e.document, e.installment, e.installments, e.group_id, e.sale_id,
        s.number AS sale_number, e.notes
    FROM fin_entries e
    LEFT JOIN fin_categories c ON c.id = e.category_id
    LEFT JOIN customers cu ON cu.id = e.customer_id
    LEFT JOIN bank_accounts a ON a.id = e.bank_account_id
    LEFT JOIN sales s ON s.id = e.sale_id
    WHERE e.deleted_at IS NULL";

fn entry_out(r: EntryRow) -> Entry {
    Entry {
        amount: from_cents(r.amount_cents),
        paid_amount: r.paid_amount_cents.map(from_cents),
        id: r.id,
        kind: r.kind,
        description: r.description,
        category_id: r.category_id,
        category_name: r.category_name,
        customer_id: r.customer_id,
        customer_name: r.customer_name,
        counterparty: r.counterparty,
        issue_date: r.issue_date,
        due_date: r.due_date,
        status: r.status,
        paid_date: r.paid_date,
        bank_account_id: r.bank_account_id,
        bank_account_name: r.bank_account_name,
        payment_method: r.payment_method,
        document: r.document,
        installment: r.installment,
        installments: r.installments,
        group_id: r.group_id,
        sale_id: r.sale_id,
        sale_number: r.sale_number,
        notes: r.notes,
    }
}

pub async fn entries(ctx: &Ctx, q: EntryQuery) -> CoreResult<Vec<Entry>> {
    let mut conn = ctx.db.acquire().await?;
    let sql = format!(
        "{ENTRY_SELECT} AND (?1 IS NULL OR e.kind = ?1) AND (?2 IS NULL OR e.status = ?2)
           AND (?3 IS NULL OR e.due_date >= ?3) AND (?4 IS NULL OR e.due_date <= ?4)
           AND (?5 IS NULL OR e.customer_id = ?5)
         ORDER BY e.due_date, e.description LIMIT 20000"
    );
    let ne = |o: &Option<String>| o.as_deref().filter(|s| !s.is_empty()).map(str::to_owned);
    let rows: Vec<EntryRow> = sqlx::query_as(&sql)
        .bind(ne(&q.kind))
        .bind(ne(&q.status))
        .bind(ne(&q.due_from))
        .bind(ne(&q.due_to))
        .bind(ne(&q.customer_id))
        .fetch_all(&mut *conn)
        .await?;
    Ok(rows.into_iter().map(entry_out).collect())
}

async fn load_entry(conn: &mut Conn, id: &str) -> CoreResult<EntryRow> {
    let sql = format!("{ENTRY_SELECT} AND e.id = ?");
    sqlx::query_as(&sql)
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| CoreError::NotFound("Título não encontrado".into()))
}

/// Cria 1 título, N parcelas (valor dividido) ou N meses recorrentes (mesmo valor).
pub async fn create_entry(ctx: &Ctx, data: EntryInput) -> CoreResult<Vec<Entry>> {
    let mut v = Validator::default();
    v.check(
        matches!(data.kind.as_str(), "receber" | "pagar"),
        "kind",
        "Tipo inválido",
    );
    v.check(
        validate::len_between(&data.description, 1, 200),
        "description",
        "Informe a descrição",
    );
    let cents = positive_cents(&mut v, "amount", data.amount);
    let due = parse_date(&data.due_date);
    v.check(due.is_some(), "due_date", "Vencimento inválido");
    let issue = match &data.issue_date {
        Some(d) => parse_date(d),
        None => Some(today()),
    };
    v.check(issue.is_some(), "issue_date", "Data inválida");
    let installments = data.installments.unwrap_or(1);
    let repeat = data.repeat_months.unwrap_or(1);
    v.check(
        (1..=120).contains(&installments),
        "installments",
        "Parcelas de 1 a 120",
    );
    v.check(
        (1..=120).contains(&repeat),
        "repeat_months",
        "Repetição de 1 a 120 meses",
    );
    v.check(
        installments == 1 || repeat == 1,
        "repeat_months",
        "Use parcelas ou repetição, não os dois",
    );
    v.finish()?;

    let mut tx = ctx.db.begin().await?;
    ensure_default_categories(&mut tx, &ctx.inst).await?;
    let ckind = category_kind(&mut tx, &data.category_id)
        .await?
        .ok_or_else(|| CoreError::field("category_id", "Categoria inválida"))?;
    let expected = if data.kind == "receber" {
        "receita"
    } else {
        "despesa"
    };
    if ckind != expected {
        return Err(CoreError::field(
            "category_id",
            "A categoria não combina com o tipo (receita/despesa)",
        ));
    }
    if let Some(cid) = &data.customer_id {
        let ok: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM customers WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(cid)
        .fetch_one(&mut *tx)
        .await?;
        if ok == 0 {
            return Err(CoreError::field("customer_id", "Cliente não encontrado"));
        }
    }

    let n = installments.max(repeat);
    let group = (n > 1).then(db::new_id);
    let (base, remainder) = if installments > 1 {
        (
            cents / i64::from(installments),
            cents % i64::from(installments),
        )
    } else {
        (cents, 0)
    };
    let due = due.unwrap_or_else(today);
    let issue = fmt(issue.unwrap_or_else(today));
    let mut ids = Vec::new();
    for k in 0..n {
        let amount = if installments > 1 && k == n - 1 {
            base + remainder
        } else {
            base
        };
        let due_k = due.checked_add_months(Months::new(k)).unwrap_or(due);
        let desc = if installments > 1 {
            format!("{} ({}/{})", data.description.trim(), k + 1, n)
        } else {
            data.description.trim().to_owned()
        };
        let id = db::new_id();
        let now = db::now();
        sqlx::query(
            "INSERT INTO fin_entries (id, kind, description, category_id, customer_id, counterparty, amount_cents,
                issue_date, due_date, document, installment, installments, group_id, notes,
                branch_id, device_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&data.kind)
        .bind(&desc)
        .bind(&data.category_id)
        .bind(&data.customer_id)
        .bind(validate::opt_text(data.counterparty.clone()))
        .bind(amount)
        .bind(&issue)
        .bind(fmt(due_k))
        .bind(validate::opt_text(data.document.clone()))
        .bind((n > 1).then_some(i64::from(k + 1)))
        .bind((n > 1).then_some(i64::from(n)))
        .bind(&group)
        .bind(validate::opt_text(data.notes.clone()))
        .bind(&ctx.inst.branch_id)
        .bind(&ctx.inst.device_id)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        ids.push(id);
    }
    audit(&mut tx, ctx, "fin_entry.create", "fin_entry", &ids[0], serde_json::json!({
        "kind": data.kind, "description": data.description.trim(), "total": from_cents(cents), "count": n,
    }))
    .await?;
    let mut out = Vec::new();
    for id in &ids {
        out.push(entry_out(load_entry(&mut tx, id).await?));
    }
    tx.commit().await?;
    Ok(out)
}

pub async fn update_entry(ctx: &Ctx, id: &str, data: EntryPatch) -> CoreResult<Entry> {
    let mut v = Validator::default();
    if let Some(d) = &data.description {
        v.check(
            validate::len_between(d, 1, 200),
            "description",
            "Informe a descrição",
        );
    }
    let amount = data.amount.map(|a| positive_cents(&mut v, "amount", a));
    let due = data.due_date.as_deref().map(parse_date);
    v.check(
        !matches!(due, Some(None)),
        "due_date",
        "Vencimento inválido",
    );
    v.finish()?;
    let mut tx = ctx.db.begin().await?;
    let cur = load_entry(&mut tx, id).await?;
    if cur.status != "aberto" {
        return Err(CoreError::BusinessRule(
            "Só títulos em aberto podem ser editados".into(),
        ));
    }
    if cur.sale_id.is_some() && (amount.is_some() || data.customer_id.is_some()) {
        return Err(CoreError::BusinessRule(
            "Título de venda: valor e cliente vêm da venda".into(),
        ));
    }
    if let Some(c) = &data.category_id {
        let k = category_kind(&mut tx, c)
            .await?
            .ok_or_else(|| CoreError::field("category_id", "Categoria inválida"))?;
        let expected = if cur.kind == "receber" {
            "receita"
        } else {
            "despesa"
        };
        if k != expected {
            return Err(CoreError::field(
                "category_id",
                "A categoria não combina com o tipo",
            ));
        }
    }
    let opt = |o: &Option<Option<String>>, curv: &Option<String>| match o {
        Some(v) => v
            .as_ref()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty()),
        None => curv.clone(),
    };
    sqlx::query(
        "UPDATE fin_entries SET description = ?, category_id = ?, customer_id = ?, counterparty = ?,
            amount_cents = ?, due_date = ?, document = ?, notes = ?, updated_at = ? WHERE id = ?",
    )
    .bind(data.description.as_deref().map(str::trim).unwrap_or(&cur.description))
    .bind(data.category_id.as_deref().unwrap_or(&cur.category_id))
    .bind(opt(&data.customer_id, &cur.customer_id))
    .bind(opt(&data.counterparty, &cur.counterparty))
    .bind(amount.unwrap_or(cur.amount_cents))
    .bind(due.flatten().map(fmt).unwrap_or(cur.due_date.clone()))
    .bind(opt(&data.document, &cur.document))
    .bind(opt(&data.notes, &cur.notes))
    .bind(db::now())
    .bind(id)
    .execute(&mut *tx)
    .await?;
    audit(
        &mut tx,
        ctx,
        "fin_entry.update",
        "fin_entry",
        id,
        serde_json::json!({
            "description": data.description, "amount": data.amount, "due_date": data.due_date,
        }),
    )
    .await?;
    let row = load_entry(&mut tx, id).await?;
    tx.commit().await?;
    Ok(entry_out(row))
}

/// Baixa: recebe/paga o título e lança na conta.
pub async fn pay_entry(ctx: &Ctx, id: &str, data: PayInput) -> CoreResult<Entry> {
    let mut v = Validator::default();
    let date = parse_date(&data.date);
    v.check(date.is_some(), "date", "Data inválida");
    let paid = data.amount.map(|a| positive_cents(&mut v, "amount", a));
    v.finish()?;
    let mut tx = ctx.db.begin().await?;
    let cur = load_entry(&mut tx, id).await?;
    if cur.status != "aberto" {
        return Err(CoreError::BusinessRule(
            "Este título não está em aberto".into(),
        ));
    }
    let account = match &data.bank_account_id {
        Some(a) => {
            active_account(&mut tx, a).await?;
            a.clone()
        }
        None => default_account(&mut tx, &ctx.inst).await?,
    };
    let paid = paid.unwrap_or(cur.amount_cents);
    let date = fmt(date.unwrap_or_else(today));
    let signed = if cur.kind == "receber" { paid } else { -paid };
    let who = cur.customer_name.clone().or(cur.counterparty.clone());
    let desc = match who {
        Some(w) => format!("{} · {w}", cur.description),
        None => cur.description.clone(),
    };
    insert_tx(
        &mut tx,
        &ctx.inst,
        NewTx {
            account_id: &account,
            date: &date,
            amount_cents: signed,
            description: &desc,
            category_id: Some(&cur.category_id),
            entry_id: Some(id),
            sale_id: None,
            transfer_group: None,
            fitid: None,
            reconciled: false,
        },
    )
    .await?;
    sqlx::query(
        "UPDATE fin_entries SET status = 'pago', paid_date = ?, paid_amount_cents = ?, bank_account_id = ?,
            payment_method = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&date)
    .bind(paid)
    .bind(&account)
    .bind(validate::opt_text(data.payment_method))
    .bind(db::now())
    .bind(id)
    .execute(&mut *tx)
    .await?;
    let action = if cur.kind == "receber" {
        "fin_entry.receive"
    } else {
        "fin_entry.pay"
    };
    audit(
        &mut tx,
        ctx,
        action,
        "fin_entry",
        id,
        serde_json::json!({"amount": from_cents(paid), "date": date}),
    )
    .await?;
    let row = load_entry(&mut tx, id).await?;
    tx.commit().await?;
    Ok(entry_out(row))
}

/// Estorna a baixa: o título volta a aberto e o lançamento sai do extrato.
pub async fn unpay_entry(ctx: &Ctx, id: &str) -> CoreResult<Entry> {
    let mut tx = ctx.db.begin().await?;
    let cur = load_entry(&mut tx, id).await?;
    if cur.status != "pago" {
        return Err(CoreError::BusinessRule(
            "Este título não está baixado".into(),
        ));
    }
    let now = db::now();
    sqlx::query("UPDATE bank_transactions SET deleted_at = ?, updated_at = ? WHERE entry_id = ? AND deleted_at IS NULL")
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE fin_entries SET status = 'aberto', paid_date = NULL, paid_amount_cents = NULL,
            bank_account_id = NULL, payment_method = NULL, updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    audit(
        &mut tx,
        ctx,
        "fin_entry.unpay",
        "fin_entry",
        id,
        serde_json::json!({}),
    )
    .await?;
    let row = load_entry(&mut tx, id).await?;
    tx.commit().await?;
    Ok(entry_out(row))
}

pub async fn cancel_entry(ctx: &Ctx, id: &str) -> CoreResult<Entry> {
    let mut tx = ctx.db.begin().await?;
    let cur = load_entry(&mut tx, id).await?;
    if cur.status != "aberto" {
        return Err(CoreError::BusinessRule(
            "Só títulos em aberto podem ser cancelados (estorne a baixa antes)".into(),
        ));
    }
    if cur.sale_id.is_some() {
        return Err(CoreError::BusinessRule(
            "Título de venda fiado: cancele a venda".into(),
        ));
    }
    sqlx::query("UPDATE fin_entries SET status = 'cancelado', updated_at = ? WHERE id = ?")
        .bind(db::now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    audit(
        &mut tx,
        ctx,
        "fin_entry.cancel",
        "fin_entry",
        id,
        serde_json::json!({}),
    )
    .await?;
    let row = load_entry(&mut tx, id).await?;
    tx.commit().await?;
    Ok(entry_out(row))
}

// ------------------------------------------------------------------ ponte com vendas

/// Venda concluída: à vista entra na conta padrão; fiado vira conta a receber (30 dias).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn on_sale_created(
    tx: &mut Conn,
    inst: &Installation,
    sale_id: &str,
    number: i64,
    payment_method: &str,
    total_cents: i64,
    customer_id: Option<&str>,
    local_date: NaiveDate,
) -> CoreResult<()> {
    if total_cents <= 0 {
        return Ok(());
    }
    ensure_default_categories(tx, inst).await?;
    let category = system_category(tx, "vendas").await?;
    let date = fmt(local_date);
    if payment_method == "fiado" {
        let due = local_date
            .checked_add_days(chrono::Days::new(30))
            .unwrap_or(local_date);
        let now = db::now();
        sqlx::query(
            "INSERT INTO fin_entries (id, kind, description, category_id, customer_id, amount_cents, issue_date,
                due_date, sale_id, branch_id, device_id, created_at, updated_at)
             VALUES (?, 'receber', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(db::new_id())
        .bind(format!("Venda nº {number} (fiado)"))
        .bind(&category)
        .bind(customer_id)
        .bind(total_cents)
        .bind(&date)
        .bind(fmt(due))
        .bind(sale_id)
        .bind(&inst.branch_id)
        .bind(&inst.device_id)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    } else {
        let account = default_account(tx, inst).await?;
        let desc = format!("Venda nº {number}");
        insert_tx(
            tx,
            inst,
            NewTx {
                account_id: &account,
                date: &date,
                amount_cents: total_cents,
                description: &desc,
                category_id: Some(&category),
                entry_id: None,
                sale_id: Some(sale_id),
                transfer_group: None,
                fitid: None,
                reconciled: false,
            },
        )
        .await?;
    }
    Ok(())
}

/// Venda cancelada: tira a entrada do extrato e cancela o fiado em aberto.
/// Fiado já recebido continua como está (o dinheiro entrou); a auditoria registra o cancelamento.
pub(crate) async fn on_sale_canceled(tx: &mut Conn, sale_id: &str) -> CoreResult<()> {
    let now = db::now();
    sqlx::query("UPDATE bank_transactions SET deleted_at = ?, updated_at = ? WHERE sale_id = ? AND deleted_at IS NULL")
        .bind(&now)
        .bind(&now)
        .bind(sale_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE fin_entries SET status = 'cancelado', updated_at = ? WHERE sale_id = ? AND status = 'aberto'")
        .bind(&now)
        .bind(sale_id)
        .execute(&mut *tx)
        .await?;
    Ok(())
}

// ------------------------------------------------------------------ resumo

/// Soma e quantidade de títulos em aberto com vencimento em [from, to) ou [from, to].
async fn open_sum(
    conn: &mut Conn,
    kind: &str,
    from: Option<&str>,
    to: &str,
    inclusive: bool,
) -> CoreResult<(i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT coalesce(sum(amount_cents), 0), count(*) FROM fin_entries
         WHERE deleted_at IS NULL AND status = 'aberto' AND kind = ?1
           AND (?2 IS NULL OR due_date >= ?2)
           AND (due_date < ?3 OR (?4 AND due_date = ?3))",
    )
    .bind(kind)
    .bind(from)
    .bind(to)
    .bind(inclusive)
    .fetch_one(conn)
    .await?)
}

/// Saldos e títulos vencidos/próximos para o painel. `today` = data local do usuário.
pub async fn summary(ctx: &Ctx, today: &str) -> CoreResult<FinanceSummary> {
    let t = parse_date(today).ok_or_else(|| CoreError::field("today", "Data inválida"))?;
    let next7 = fmt(t.checked_add_days(chrono::Days::new(7)).unwrap_or(t));
    let today = fmt(t);
    let mut conn = ctx.db.acquire().await?;
    let total: i64 = sqlx::query_scalar(
        "SELECT coalesce(sum(a.initial_balance_cents), 0) + coalesce((SELECT sum(t.amount_cents) FROM bank_transactions t
            JOIN bank_accounts b ON b.id = t.bank_account_id
            WHERE t.deleted_at IS NULL AND b.deleted_at IS NULL AND b.active = 1), 0)
         FROM bank_accounts a WHERE a.deleted_at IS NULL AND a.active = 1",
    )
    .fetch_one(&mut *conn)
    .await?;
    let (r_over, r_over_n) = open_sum(&mut conn, "receber", None, &today, false).await?;
    let (r_next, _) = open_sum(&mut conn, "receber", Some(&today), &next7, true).await?;
    let (p_over, p_over_n) = open_sum(&mut conn, "pagar", None, &today, false).await?;
    let (p_next, _) = open_sum(&mut conn, "pagar", Some(&today), &next7, true).await?;
    let (p_today, _) = open_sum(&mut conn, "pagar", Some(&today), &today, true).await?;
    Ok(FinanceSummary {
        total_balance: from_cents(total),
        receivable_overdue: from_cents(r_over),
        receivable_overdue_count: r_over_n,
        receivable_next7: from_cents(r_next),
        payable_overdue: from_cents(p_over),
        payable_overdue_count: p_over_n,
        payable_next7: from_cents(p_next),
        payable_today: from_cents(p_today),
    })
}
