//! Clientes. Cadastro simples (sem repository próprio: SQL curto e só usado aqui).

use crate::audit::{self, Entry};
use crate::db::{self, Conn};
use crate::error::{CoreError, CoreResult, Validator};
use crate::validate;
use crate::Ctx;

use super::models::{Customer, CustomerInput, CustomerPatch};

const COLUMNS: &str =
    "id, name, document, phone, email, city, state, notes, active, created_at, updated_at";

pub async fn list(ctx: &Ctx) -> CoreResult<Vec<Customer>> {
    let mut conn = ctx.db.acquire().await?;
    let sql = format!(
        "SELECT {COLUMNS} FROM customers WHERE deleted_at IS NULL ORDER BY name COLLATE NOCASE"
    );
    Ok(sqlx::query_as(&sql).fetch_all(&mut *conn).await?)
}

pub(crate) async fn get(conn: &mut Conn, id: &str) -> CoreResult<Option<Customer>> {
    let sql = format!("SELECT {COLUMNS} FROM customers WHERE id = ? AND deleted_at IS NULL");
    Ok(sqlx::query_as(&sql).bind(id).fetch_optional(conn).await?)
}

/// Normaliza e valida os campos opcionais; erros vão para `v`.
struct Clean {
    document: Option<String>,
    email: Option<String>,
    state: Option<String>,
}

fn clean(
    v: &mut Validator,
    document: Option<&str>,
    email: Option<&str>,
    state: Option<&str>,
) -> Clean {
    let document = validate::document(document);
    v.check(document.is_ok(), "document", "CPF ou CNPJ inválido");
    let email = email.map(str::trim).filter(|e| !e.is_empty());
    let parsed_email = email.and_then(validate::email);
    v.check(
        email.is_none() || parsed_email.is_some(),
        "email",
        "E-mail inválido",
    );
    let state = state.map(str::trim).filter(|s| !s.is_empty());
    let parsed_state = state.and_then(validate::uf);
    v.check(
        state.is_none() || parsed_state.is_some(),
        "state",
        "UF inválida (ex.: SP)",
    );
    Clean {
        document: document.unwrap_or_default(),
        email: parsed_email,
        state: parsed_state,
    }
}

async fn check_document_free(
    conn: &mut Conn,
    document: Option<&str>,
    except: Option<&str>,
) -> CoreResult<()> {
    let Some(doc) = document else { return Ok(()) };
    let other: Option<String> =
        sqlx::query_scalar("SELECT id FROM customers WHERE document = ? AND deleted_at IS NULL")
            .bind(doc)
            .fetch_optional(conn)
            .await?;
    match other {
        Some(id) if Some(id.as_str()) != except => Err(CoreError::Conflict(
            "Já existe um cliente com esse CPF/CNPJ".into(),
        )),
        _ => Ok(()),
    }
}

async fn save(conn: &mut Conn, ctx: &Ctx, c: &Customer, insert: bool) -> CoreResult<()> {
    let sql = if insert {
        "INSERT INTO customers (name, document, phone, email, city, state, notes, active, updated_at,
            id, created_at, branch_id, device_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    } else {
        "UPDATE customers SET name = ?, document = ?, phone = ?, email = ?, city = ?, state = ?,
            notes = ?, active = ?, updated_at = ? WHERE id = ?"
    };
    let mut q = sqlx::query(sql)
        .bind(&c.name)
        .bind(&c.document)
        .bind(&c.phone)
        .bind(&c.email)
        .bind(&c.city)
        .bind(&c.state)
        .bind(&c.notes)
        .bind(c.active)
        .bind(&c.updated_at)
        .bind(&c.id);
    if insert {
        q = q
            .bind(&c.created_at)
            .bind(&ctx.inst.branch_id)
            .bind(&ctx.inst.device_id);
    }
    q.execute(conn).await?;
    Ok(())
}

pub async fn create(ctx: &Ctx, data: CustomerInput) -> CoreResult<Customer> {
    let mut v = Validator::default();
    v.check(
        validate::len_between(&data.name, 2, 200),
        "name",
        "Informe o nome (2 a 200 letras)",
    );
    let c = clean(
        &mut v,
        data.document.as_deref(),
        data.email.as_deref(),
        data.state.as_deref(),
    );
    v.finish()?;

    let mut tx = ctx.db.begin().await?;
    check_document_free(&mut tx, c.document.as_deref(), None).await?;
    let now = db::now();
    let customer = Customer {
        id: db::new_id(),
        name: data.name.trim().to_owned(),
        document: c.document,
        phone: validate::opt_text(data.phone),
        email: c.email,
        city: validate::opt_text(data.city),
        state: c.state,
        notes: validate::opt_text(data.notes),
        active: true,
        created_at: now.clone(),
        updated_at: now,
    };
    save(&mut tx, ctx, &customer, true).await?;
    let entry = Entry {
        actor_id: Some(&ctx.user.id),
        action: "customer.create",
        entity: "customer",
        entity_id: Some(&customer.id),
        before: None,
        after: Some(audit::snapshot(&customer)),
    };
    audit::record(&mut tx, &ctx.inst, entry).await?;
    tx.commit().await?;
    Ok(customer)
}

pub async fn update(ctx: &Ctx, id: &str, data: CustomerPatch) -> CoreResult<Customer> {
    let mut v = Validator::default();
    let mut tx = ctx.db.begin().await?;
    let before = get(&mut tx, id)
        .await?
        .ok_or_else(|| CoreError::NotFound("Cliente não encontrado".into()))?;
    let mut c = before.clone();
    if let Some(name) = data.name {
        v.check(
            validate::len_between(&name, 2, 200),
            "name",
            "Informe o nome (2 a 200 letras)",
        );
        c.name = name.trim().to_owned();
    }
    let cl = clean(
        &mut v,
        data.document.as_ref().and_then(|d| d.as_deref()),
        data.email.as_ref().and_then(|e| e.as_deref()),
        data.state.as_ref().and_then(|s| s.as_deref()),
    );
    if data.document.is_some() {
        c.document = cl.document;
    }
    if data.email.is_some() {
        c.email = cl.email;
    }
    if data.state.is_some() {
        c.state = cl.state;
    }
    if let Some(phone) = data.phone {
        c.phone = validate::opt_text(phone);
    }
    if let Some(city) = data.city {
        c.city = validate::opt_text(city);
    }
    if let Some(notes) = data.notes {
        c.notes = validate::opt_text(notes);
    }
    if let Some(active) = data.active {
        c.active = active;
    }
    v.finish()?;
    check_document_free(&mut tx, c.document.as_deref(), Some(id)).await?;

    c.updated_at = db::now();
    save(&mut tx, ctx, &c, false).await?;
    let entry = Entry {
        actor_id: Some(&ctx.user.id),
        action: "customer.update",
        entity: "customer",
        entity_id: Some(id),
        before: Some(audit::snapshot(&before)),
        after: Some(audit::snapshot(&c)),
    };
    audit::record_changes(&mut tx, &ctx.inst, entry).await?;
    tx.commit().await?;
    Ok(c)
}
