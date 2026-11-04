use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use serde_json::{json, Value};

use crate::audit::{self, Entry};
use crate::db::{self, Conn, Installation};
use crate::error::{CoreError, CoreResult};
use crate::modules::customers::service as customers;
use crate::modules::finance::service as finance;
use crate::modules::integrations::models::Job;
use crate::modules::integrations::repository::{self as jobs_repo, NewJob};
use crate::modules::integrations::service as integrations;
use crate::money;
use crate::{Core, Ctx};

use super::models::{ChargeRow, Payer, PixCharge, PixProviderOption};

/// Validade da cobrança (o Mercado Pago exige pelo menos 30 min).
pub const EXPIRES_SECS: i64 = 3600;
/// Entidade de referência dos trabalhos da fila.
pub const REF_ENTITY: &str = "pix_charges";

const NOT_FOUND: &str = "Cobrança PIX não encontrada.";
const ACTIVE: [&str; 2] = ["gerando", "aguardando"];

const SELECT: &str =
    "SELECT c.id, c.sale_id, s.number AS sale_number, c.entry_id, c.integration_id,
        c.provider, c.amount_cents, c.status, c.txid, c.provider_ref, c.copy_paste, c.expires_at,
        c.paid_at, c.error, c.created_at
    FROM pix_charges c JOIN sales s ON s.id = c.sale_id
    WHERE c.deleted_at IS NULL";

async fn load(conn: &mut Conn, id: &str) -> CoreResult<Option<ChargeRow>> {
    let sql = format!("{SELECT} AND c.id = ?");
    Ok(sqlx::query_as(&sql).bind(id).fetch_optional(conn).await?)
}

async fn latest_for_sale(conn: &mut Conn, sale_id: &str) -> CoreResult<Option<ChargeRow>> {
    let sql = format!("{SELECT} AND c.sale_id = ? ORDER BY c.created_at DESC, c.id DESC LIMIT 1");
    Ok(sqlx::query_as(&sql)
        .bind(sale_id)
        .fetch_optional(conn)
        .await?)
}

/// Identificador da cobrança no padrão do Banco Central: 26 a 35 letras/números.
fn txid_for(charge_id: &str) -> String {
    let hex: String = charge_id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    format!("NUC{hex}").chars().take(35).collect()
}

/// QR Code em SVG, desenhado aqui a partir do copia-e-cola (preto no branco, como os bancos leem).
pub fn qr_svg(copy_paste: &str) -> Option<String> {
    use qrcode::render::svg;
    let code = qrcode::QrCode::with_error_correction_level(copy_paste, qrcode::EcLevel::M).ok()?;
    Some(
        code.render::<svg::Color>()
            .min_dimensions(240, 240)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build(),
    )
}

fn to_out(core: &Core, sandbox: bool, r: ChargeRow) -> PixCharge {
    let provider_name = core
        .provider(&r.provider)
        .map(|p| p.info().name.to_owned())
        .unwrap_or_else(|| r.provider.clone());
    let show_code = r.status == "aguardando";
    PixCharge {
        qr_svg: r
            .copy_paste
            .as_deref()
            .filter(|_| show_code)
            .and_then(qr_svg),
        copy_paste: r.copy_paste.filter(|_| show_code),
        id: r.id,
        sale_id: r.sale_id,
        sale_number: r.sale_number,
        provider: r.provider,
        provider_name,
        amount: money::from_cents(r.amount_cents),
        status: r.status,
        expires_at: r.expires_at,
        paid_at: r.paid_at,
        error: r.error,
        sandbox,
        created_at: r.created_at,
    }
}

async fn out(core: &Core, conn: &mut Conn, r: ChargeRow) -> CoreResult<PixCharge> {
    let env: Option<String> =
        sqlx::query_scalar("SELECT environment FROM integrations WHERE id = ?")
            .bind(&r.integration_id)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(to_out(core, env.as_deref() != Some("production"), r))
}

fn local_date(at: &str) -> NaiveDate {
    // sem usuário na tela (fila em segundo plano): data de Brasília
    let brt = FixedOffset::west_opt(3 * 3600).expect("fuso válido");
    DateTime::parse_from_rfc3339(at)
        .map(|d| d.with_timezone(&brt).date_naive())
        .unwrap_or_else(|_| Utc::now().with_timezone(&brt).date_naive())
}

// ------------------------------------------------------------------ criação

/// Abre a cobrança e põe o `pix.create` na fila, na transação da venda. Funciona sem internet.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn start_in(
    tx: &mut Conn,
    inst: &Installation,
    actor_id: Option<&str>,
    sale_id: &str,
    sale_number: i64,
    entry_id: &str,
    provider: &str,
    amount_cents: i64,
    customer_id: Option<&str>,
) -> CoreResult<String> {
    let integration = jobs_repo::by_provider(tx, provider)
        .await?
        .filter(|i| i.enabled)
        .ok_or_else(|| {
            CoreError::field(
                "pix_provider",
                "Este serviço de PIX não está ativado. Configure-o em Integrações.",
            )
        })?;
    if amount_cents <= 0 {
        return Err(CoreError::BusinessRule(
            "Não dá para gerar PIX de valor zero.".into(),
        ));
    }
    let payer = match customer_id {
        Some(cid) => customers::get(tx, cid).await?.map(|c| Payer {
            name: c.name,
            document: c.document,
            email: c.email,
        }),
        None => None,
    };
    let id = db::new_id();
    let txid = txid_for(&id);
    let now = db::now();
    sqlx::query(
        "INSERT INTO pix_charges (id, sale_id, entry_id, integration_id, provider, amount_cents, status,
            txid, branch_id, device_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, 'gerando', ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(sale_id)
    .bind(entry_id)
    .bind(&integration.id)
    .bind(provider)
    .bind(amount_cents)
    .bind(&txid)
    .bind(&inst.branch_id)
    .bind(&inst.device_id)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    let payload = json!({
        "charge_id": id,
        "txid": txid,
        "amount_cents": amount_cents,
        "description": format!("Venda nº {sale_number}"),
        "expires_secs": EXPIRES_SECS,
        "payer": payer,
    });
    integrations::enqueue(
        tx,
        inst,
        provider,
        "pix.create",
        &payload,
        Some((REF_ENTITY, &id)),
    )
    .await?;
    audit::record(
        tx,
        inst,
        Entry {
            actor_id,
            action: "pix_charge.create",
            entity: "pix_charge",
            entity_id: Some(&id),
            before: None,
            after: Some(json!({
                "sale_number": sale_number,
                "provider": provider,
                "amount": money::from_cents(amount_cents),
            })),
        },
    )
    .await?;
    Ok(id)
}

// ------------------------------------------------------------------ resultado da fila

/// Efeito local de um trabalho `pix.*` concluído (chamado pela fila, na mesma transação).
pub(crate) async fn on_job_done(
    tx: &mut Conn,
    inst: &Installation,
    job: &Job,
    result: &Value,
) -> CoreResult<()> {
    let Some(charge_id) = job.payload["charge_id"].as_str() else {
        return Ok(());
    };
    let Some(c) = load(tx, charge_id).await? else {
        return Ok(());
    };
    let now = db::now();
    match job.kind.as_str() {
        "pix.create" => {
            let provider_ref = result["provider_ref"].as_str().map(str::to_owned);
            let copy_paste = result["copy_paste"].as_str().map(str::to_owned);
            let expires_at = result["expires_at"].as_str().map(str::to_owned);
            if c.status != "gerando" {
                // a venda/cobrança foi cancelada enquanto o PIX era gerado: desfaz no provedor
                if let Some(r) = &provider_ref {
                    enqueue_cancel(tx, inst, &c, r).await?;
                }
                return Ok(());
            }
            if copy_paste.is_none() {
                sqlx::query(
                    "UPDATE pix_charges SET status = 'erro', error = ?, updated_at = ? WHERE id = ?",
                )
                .bind("O serviço não devolveu o código PIX.")
                .bind(&now)
                .bind(&c.id)
                .execute(&mut *tx)
                .await?;
                return Ok(());
            }
            sqlx::query(
                "UPDATE pix_charges SET status = 'aguardando', provider_ref = ?, copy_paste = ?,
                    expires_at = ?, error = NULL, updated_at = ? WHERE id = ?",
            )
            .bind(&provider_ref)
            .bind(&copy_paste)
            .bind(&expires_at)
            .bind(&now)
            .bind(&c.id)
            .execute(&mut *tx)
            .await?;
            let payload = json!({
                "charge_id": c.id,
                "provider_ref": provider_ref,
                "txid": c.txid,
                "created_at": now,
                "expires_at": expires_at,
            });
            jobs_repo::insert_job(
                tx,
                inst,
                NewJob {
                    integration_id: &c.integration_id,
                    kind: "pix.poll",
                    payload: &payload,
                    ref_entity: Some(REF_ENTITY),
                    ref_id: Some(&c.id),
                },
            )
            .await?;
        }
        "pix.poll" => match result["status"].as_str() {
            Some("paid") => {
                let paid_at = result["paid_at"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| now.clone());
                let mut note = None;
                if c.status == "aguardando" {
                    let received = finance::receive_automatically(
                        tx,
                        inst,
                        &c.entry_id,
                        local_date(&paid_at),
                        "pix",
                        "pix automático",
                    )
                    .await?;
                    if !received {
                        note = Some(
                            "Pago pelo PIX, mas o título já tinha sido baixado à mão. Confira o financeiro.",
                        );
                    }
                } else {
                    note = Some(
                        "O PIX foi pago depois do cancelamento. Devolva o valor pelo app do serviço.",
                    );
                }
                sqlx::query(
                    "UPDATE pix_charges SET status = 'pago', paid_at = ?, error = ?, updated_at = ?
                     WHERE id = ?",
                )
                .bind(&paid_at)
                .bind(note)
                .bind(&now)
                .bind(&c.id)
                .execute(&mut *tx)
                .await?;
                audit::record(
                    tx,
                    inst,
                    Entry {
                        actor_id: None,
                        action: "pix_charge.paid",
                        entity: "pix_charge",
                        entity_id: Some(&c.id),
                        before: None,
                        after: Some(json!({
                            "sale_number": c.sale_number,
                            "amount": money::from_cents(c.amount_cents),
                            "paid_at": paid_at,
                        })),
                    },
                )
                .await?;
            }
            Some(other) if ACTIVE.contains(&c.status.as_str()) => {
                let status = if other == "canceled" {
                    "cancelado"
                } else {
                    "expirado"
                };
                sqlx::query("UPDATE pix_charges SET status = ?, updated_at = ? WHERE id = ?")
                    .bind(status)
                    .bind(&now)
                    .bind(&c.id)
                    .execute(&mut *tx)
                    .await?;
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

/// Trabalho `pix.*` que desistiu: a cobrança mostra o motivo.
pub(crate) async fn on_job_failed(tx: &mut Conn, job: &Job, message: &str) -> CoreResult<()> {
    let Some(charge_id) = job.payload["charge_id"].as_str() else {
        return Ok(());
    };
    let now = db::now();
    match job.kind.as_str() {
        "pix.create" => {
            sqlx::query(
                "UPDATE pix_charges SET status = 'erro', error = ?, updated_at = ?
                 WHERE id = ? AND status = 'gerando'",
            )
            .bind(message)
            .bind(&now)
            .bind(charge_id)
            .execute(&mut *tx)
            .await?;
        }
        // a consulta parou: o PIX pode ter sido pago; a cobrança continua aguardando
        "pix.poll" => {
            sqlx::query("UPDATE pix_charges SET error = ?, updated_at = ? WHERE id = ?")
                .bind(format!(
                    "Não foi possível confirmar o pagamento: {message} Tente de novo em Integrações."
                ))
                .bind(&now)
                .bind(charge_id)
                .execute(&mut *tx)
                .await?;
        }
        _ => {}
    }
    Ok(())
}

async fn enqueue_cancel(
    tx: &mut Conn,
    inst: &Installation,
    c: &ChargeRow,
    provider_ref: &str,
) -> CoreResult<()> {
    let payload = json!({ "charge_id": c.id, "provider_ref": provider_ref, "txid": c.txid });
    jobs_repo::insert_job(
        tx,
        inst,
        NewJob {
            integration_id: &c.integration_id,
            kind: "pix.cancel",
            payload: &payload,
            ref_entity: Some(REF_ENTITY),
            ref_id: Some(&c.id),
        },
    )
    .await?;
    Ok(())
}

/// Cancela a cobrança localmente, para a consulta e pede o cancelamento no provedor.
async fn cancel_in(tx: &mut Conn, inst: &Installation, c: &ChargeRow) -> CoreResult<()> {
    let now = db::now();
    sqlx::query("UPDATE pix_charges SET status = 'cancelado', updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&c.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE integration_jobs SET status = 'canceled', finished_at = ?, updated_at = ?
         WHERE ref_entity = ? AND ref_id = ? AND status IN ('pending', 'failed')",
    )
    .bind(&now)
    .bind(&now)
    .bind(REF_ENTITY)
    .bind(&c.id)
    .execute(&mut *tx)
    .await?;
    if let Some(r) = &c.provider_ref {
        enqueue_cancel(tx, inst, c, r).await?;
    }
    Ok(())
}

/// Venda cancelada: cancela as cobranças ainda abertas.
pub(crate) async fn on_sale_canceled(
    tx: &mut Conn,
    inst: &Installation,
    sale_id: &str,
) -> CoreResult<()> {
    let sql = format!("{SELECT} AND c.sale_id = ? AND c.status IN ('gerando', 'aguardando')");
    let open: Vec<ChargeRow> = sqlx::query_as(&sql)
        .bind(sale_id)
        .fetch_all(&mut *tx)
        .await?;
    for c in &open {
        cancel_in(tx, inst, c).await?;
    }
    Ok(())
}

// ------------------------------------------------------------------ telas

/// Serviços de PIX ativados nesta instalação (para escolher na venda).
pub async fn providers(core: &Core, ctx: &Ctx) -> CoreResult<Vec<PixProviderOption>> {
    let mut conn = ctx.db.acquire().await?;
    let saved = jobs_repo::list(&mut conn).await?;
    Ok(saved
        .into_iter()
        .filter(|i| i.enabled)
        .filter_map(|i| {
            let info = core.provider(&i.provider)?.info();
            info.capabilities
                .contains(&"pix")
                .then(|| PixProviderOption {
                    id: i.provider.clone(),
                    name: info.name.to_owned(),
                    sandbox: i.environment != "production",
                    checked: i.status == "ok",
                })
        })
        .collect())
}

/// Última cobrança da venda (ou nenhuma).
pub async fn for_sale(core: &Core, ctx: &Ctx, sale_id: &str) -> CoreResult<Option<PixCharge>> {
    let mut conn = ctx.db.acquire().await?;
    match latest_for_sale(&mut conn, sale_id).await? {
        Some(r) => Ok(Some(out(core, &mut conn, r).await?)),
        None => Ok(None),
    }
}

/// "Já pagou?": roda agora a geração/consulta pendente desta cobrança e devolve a situação.
pub async fn check(core: &Core, ctx: &Ctx, id: &str) -> CoreResult<PixCharge> {
    let mut conn = ctx.db.acquire().await?;
    let c = load(&mut conn, id)
        .await?
        .ok_or_else(|| CoreError::NotFound(NOT_FOUND.into()))?;
    if ACTIVE.contains(&c.status.as_str()) {
        let pending: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM integration_jobs
             WHERE ref_entity = ? AND ref_id = ? AND status = 'pending' AND deleted_at IS NULL
             ORDER BY created_at",
        )
        .bind(REF_ENTITY)
        .bind(id)
        .fetch_all(&mut *conn)
        .await?;
        drop(conn);
        for job in pending {
            integrations::run_job(core, &job).await?;
        }
        conn = ctx.db.acquire().await?;
    }
    let c = load(&mut conn, id)
        .await?
        .ok_or_else(|| CoreError::NotFound(NOT_FOUND.into()))?;
    out(core, &mut conn, c).await
}

/// Desiste da cobrança (ex.: o cliente vai pagar de outro jeito). O título continua a receber.
pub async fn cancel(core: &Core, ctx: &Ctx, id: &str) -> CoreResult<PixCharge> {
    let mut tx = ctx.db.begin().await?;
    let c = load(&mut tx, id)
        .await?
        .ok_or_else(|| CoreError::NotFound(NOT_FOUND.into()))?;
    if !ACTIVE.contains(&c.status.as_str()) {
        return Err(CoreError::BusinessRule(
            "Esta cobrança não está mais em aberto.".into(),
        ));
    }
    cancel_in(&mut tx, &ctx.inst, &c).await?;
    audit::record(
        &mut tx,
        &ctx.inst,
        Entry {
            actor_id: Some(&ctx.user.id),
            action: "pix_charge.cancel",
            entity: "pix_charge",
            entity_id: Some(id),
            before: Some(json!({ "status": c.status })),
            after: Some(json!({ "status": "cancelado", "sale_number": c.sale_number })),
        },
    )
    .await?;
    let c = load(&mut tx, id)
        .await?
        .ok_or_else(|| CoreError::NotFound(NOT_FOUND.into()))?;
    let o = out(core, &mut tx, c).await?;
    tx.commit().await?;
    Ok(o)
}

/// Nova cobrança para uma venda PIX ainda não recebida (a anterior expirou, falhou ou foi
/// cancelada), no mesmo ou em outro serviço.
pub async fn new_for_sale(
    core: &Core,
    ctx: &Ctx,
    sale_id: &str,
    provider: &str,
) -> CoreResult<PixCharge> {
    let mut tx = ctx.db.begin().await?;
    let sale: Option<(i64, String, String, Option<String>)> = sqlx::query_as(
        "SELECT number, status, payment_method, customer_id FROM sales
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(sale_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (number, status, method, customer_id) =
        sale.ok_or_else(|| CoreError::NotFound("Venda não encontrada".into()))?;
    if status != "concluida" || method != "pix" {
        return Err(CoreError::BusinessRule(
            "Só dá para gerar PIX de uma venda PIX concluída.".into(),
        ));
    }
    if let Some(c) = latest_for_sale(&mut tx, sale_id).await? {
        if ACTIVE.contains(&c.status.as_str()) {
            return Err(CoreError::BusinessRule(
                "Já existe uma cobrança PIX em aberto para esta venda.".into(),
            ));
        }
    }
    let entry: Option<(String, i64)> = sqlx::query_as(
        "SELECT id, amount_cents FROM fin_entries
         WHERE sale_id = ? AND status = 'aberto' AND kind = 'receber' AND deleted_at IS NULL
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(sale_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (entry_id, amount) = entry.ok_or_else(|| {
        CoreError::BusinessRule("Esta venda já foi recebida; não há o que cobrar.".into())
    })?;
    let id = start_in(
        &mut tx,
        &ctx.inst,
        Some(&ctx.user.id),
        sale_id,
        number,
        &entry_id,
        provider,
        amount,
        customer_id.as_deref(),
    )
    .await?;
    let c = load(&mut tx, &id)
        .await?
        .ok_or_else(|| CoreError::Internal("cobrança recém-criada sumiu".into()))?;
    let o = out(core, &mut tx, c).await?;
    tx.commit().await?;
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn txid_follows_central_bank_rules() {
        let t = txid_for(&db::new_id());
        assert!((26..=35).contains(&t.len()), "{t}");
        assert!(t.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn qr_is_svg() {
        let svg = qr_svg("00020126580014br.gov.bcb.pix0136teste").unwrap();
        assert!(svg.contains("<svg") && svg.contains("</svg>"));
    }

    #[test]
    fn background_date_uses_brasilia() {
        assert_eq!(
            local_date("2026-11-05T02:00:00.000Z"),
            NaiveDate::from_ymd_opt(2026, 11, 4).unwrap()
        );
    }
}
