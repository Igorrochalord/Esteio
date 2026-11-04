//! PIX na venda: cobrança pela fila, título a receber até o pagamento, baixa automática,
//! cancelamentos e falhas. Usa o Simulador e um provedor falso (sem internet).

mod common;

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use common::*;
use nucleo_core::modules::integrations::catalog::{Kind, ProviderInfo};
use nucleo_core::modules::integrations::models::{IntegrationSave, Job};
use nucleo_core::modules::integrations::provider::{Call, Outcome, Provider, ProviderError};
use nucleo_core::modules::integrations::service as integrations;
use nucleo_core::modules::pix::service as pix;
use nucleo_core::modules::products::models::{Product, ProductInput};
use nucleo_core::modules::products::service as products;
use nucleo_core::modules::sales::models::{SaleInput, SaleItemInput};
use nucleo_core::modules::sales::service as sales;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::{Core, CoreError, Ctx};
use rust_decimal::Decimal;
use serde_json::{json, Map};

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// Provedor que sempre recusa a criação (dados inválidos).
struct Recusa;

#[async_trait]
impl Provider for Recusa {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            capabilities: vec!["pix"],
            ..ProviderInfo::new("recusa", "Recusa", Kind::Pix, "teste")
        }
    }
    async fn check(&self, _call: &Call<'_>) -> Result<String, ProviderError> {
        Ok("ok".into())
    }
    async fn run(&self, _call: &Call<'_>, _job: &Job) -> Outcome {
        Outcome::Error(ProviderError::Rejected("chave Pix não cadastrada".into()))
    }
}

async fn manager(core: &Core) -> Ctx {
    core.require(Module::Integrations, Action::Manage)
        .await
        .unwrap()
}

/// Simulador ativado: paga depois de `secs` segundos.
async fn simulator(core: &Core, secs: &str) {
    let mut config = Map::new();
    config.insert("segundos_para_pagar".into(), json!(secs));
    let save = IntegrationSave {
        provider: "simulador".into(),
        environment: "sandbox".into(),
        enabled: true,
        config,
        secrets: Map::new(),
    };
    integrations::save(core, &manager(core).await, save)
        .await
        .unwrap();
}

async fn bread(core: &Core) -> Product {
    let ctx = manager(core).await;
    products::create(
        &ctx,
        ProductInput {
            code: String::new(),
            barcode: None,
            name: "Bolo".into(),
            category_id: None,
            unit: None,
            price: d("19.90"),
            cost: Some(d("8")),
            track_stock: None,
            min_stock: None,
            initial_stock: Some(d("10")),
        },
    )
    .await
    .unwrap()
}

fn sale(p: &Product, payment: &str, provider: Option<&str>) -> SaleInput {
    SaleInput {
        customer_id: None,
        seller_id: None,
        payment_method: payment.into(),
        discount: None,
        notes: None,
        sold_at: None,
        tz_offset_minutes: Some(-180),
        pix_provider: provider.map(str::to_owned),
        items: vec![SaleItemInput {
            product_id: p.id.clone(),
            qty: d("1"),
            unit_price: None,
            discount: None,
        }],
    }
}

async fn entry_status(core: &Core, sale_id: &str) -> (String, i64) {
    sqlx::query_as("SELECT status, amount_cents FROM fin_entries WHERE sale_id = ?")
        .bind(sale_id)
        .fetch_one(&core.pool())
        .await
        .unwrap()
}

async fn bank_total(core: &Core, entry_or_sale: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT coalesce(sum(t.amount_cents), 0) FROM bank_transactions t
         LEFT JOIN fin_entries e ON e.id = t.entry_id
         WHERE t.deleted_at IS NULL AND (t.sale_id = ?1 OR e.sale_id = ?1)",
    )
    .bind(entry_or_sale)
    .fetch_one(&core.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn pix_sale_waits_then_receives_automatically() {
    let (_dir, core, _) = onboarded().await;
    simulator(&core, "0").await;
    let p = bread(&core).await;
    let ctx = manager(&core).await;

    // aparece como opção na venda
    let options = pix::providers(&core, &ctx).await.unwrap();
    assert_eq!(options.len(), 1);
    assert!(options[0].sandbox);

    let s = sales::create(&ctx, sale(&p, "pix", Some("simulador")))
        .await
        .unwrap();
    // ainda não recebido: título em aberto e nada no caixa
    assert_eq!(entry_status(&core, &s.id).await, ("aberto".into(), 1990));
    assert_eq!(bank_total(&core, &s.id).await, 0);
    let c = pix::for_sale(&core, &ctx, &s.id).await.unwrap().unwrap();
    assert_eq!(c.status, "gerando");
    assert!(c.qr_svg.is_none());

    // "já pagou?" gera o código e mostra o QR
    let c = pix::check(&core, &ctx, &c.id).await.unwrap();
    assert_eq!(c.status, "aguardando");
    assert!(c.copy_paste.as_deref().unwrap().contains("540519.90"));
    assert!(c.qr_svg.as_deref().unwrap().contains("<svg"));
    assert!(c.sandbox);

    // a próxima consulta confirma: baixa automática na conta padrão
    let c = pix::check(&core, &ctx, &c.id).await.unwrap();
    assert_eq!(c.status, "pago");
    assert!(c.paid_at.is_some() && c.qr_svg.is_none());
    assert_eq!(entry_status(&core, &s.id).await.0, "pago");
    assert_eq!(bank_total(&core, &s.id).await, 1990);
    let method: String =
        sqlx::query_scalar("SELECT payment_method FROM fin_entries WHERE sale_id = ?")
            .bind(&s.id)
            .fetch_one(&core.pool())
            .await
            .unwrap();
    assert_eq!(method, "pix");
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs WHERE action IN ('pix_charge.create', 'pix_charge.paid')",
    )
    .fetch_one(&core.pool())
    .await
    .unwrap();
    assert_eq!(audited, 2);
}

#[tokio::test]
async fn pix_without_provider_is_cash_and_validation() {
    let (_dir, core, _) = onboarded().await;
    let p = bread(&core).await;
    let ctx = manager(&core).await;

    // PIX conferido no app do banco: entra direto no caixa (como antes)
    let s = sales::create(&ctx, sale(&p, "pix", None)).await.unwrap();
    assert_eq!(bank_total(&core, &s.id).await, 1990);
    assert!(pix::for_sale(&core, &ctx, &s.id).await.unwrap().is_none());

    // cobrança só em PIX, e só com serviço ativado
    assert!(matches!(
        sales::create(&ctx, sale(&p, "dinheiro", Some("simulador"))).await,
        Err(CoreError::Validation(f)) if f.contains_key("pix_provider")
    ));
    assert!(matches!(
        sales::create(&ctx, sale(&p, "pix", Some("simulador"))).await,
        Err(CoreError::Validation(f)) if f.contains_key("pix_provider")
    ));
    // nada ficou pela metade (estoque intacto)
    let stock: i64 = sqlx::query_scalar("SELECT stock_milli FROM products WHERE id = ?")
        .bind(&p.id)
        .fetch_one(&core.pool())
        .await
        .unwrap();
    assert_eq!(stock, 9000);
}

#[tokio::test]
async fn canceling_the_sale_cancels_the_charge() {
    let (_dir, core, _) = onboarded().await;
    simulator(&core, "3600").await;
    let p = bread(&core).await;
    let ctx = manager(&core).await;
    let s = sales::create(&ctx, sale(&p, "pix", Some("simulador")))
        .await
        .unwrap();
    let c = pix::for_sale(&core, &ctx, &s.id).await.unwrap().unwrap();
    let c = pix::check(&core, &ctx, &c.id).await.unwrap();
    assert_eq!(c.status, "aguardando");
    let c = pix::check(&core, &ctx, &c.id).await.unwrap();
    assert_eq!(c.status, "aguardando"); // não pago ainda

    let sales_ctx = core.require(Module::Sales, Action::Manage).await.unwrap();
    sales::cancel(&sales_ctx, &s.id, "cliente desistiu")
        .await
        .unwrap();
    let c = pix::for_sale(&core, &ctx, &s.id).await.unwrap().unwrap();
    assert_eq!(c.status, "cancelado");
    assert_eq!(entry_status(&core, &s.id).await.0, "cancelado");
    let jobs: Vec<(String, String)> = sqlx::query_as(
        "SELECT kind, status FROM integration_jobs WHERE ref_id = ? ORDER BY created_at",
    )
    .bind(&c.id)
    .fetch_all(&core.pool())
    .await
    .unwrap();
    assert_eq!(
        jobs,
        vec![
            ("pix.create".into(), "done".into()),
            ("pix.poll".into(), "canceled".into()),
            ("pix.cancel".into(), "pending".into()),
        ]
    );
    // o cancelamento no provedor roda na fila
    integrations::tick(&core).await.unwrap();
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM integration_jobs WHERE status = 'pending' AND ref_id = ?",
    )
    .bind(&c.id)
    .fetch_one(&core.pool())
    .await
    .unwrap();
    assert_eq!(pending, 0);
}

#[tokio::test]
async fn cancel_charge_and_generate_a_new_one() {
    let (_dir, core, _) = onboarded().await;
    simulator(&core, "0").await;
    let p = bread(&core).await;
    let ctx = manager(&core).await;
    let s = sales::create(&ctx, sale(&p, "pix", Some("simulador")))
        .await
        .unwrap();
    let first = pix::for_sale(&core, &ctx, &s.id).await.unwrap().unwrap();

    // já existe uma em aberto
    assert!(matches!(
        pix::new_for_sale(&core, &ctx, &s.id, "simulador").await,
        Err(CoreError::BusinessRule(_))
    ));
    let canceled = pix::cancel(&core, &ctx, &first.id).await.unwrap();
    assert_eq!(canceled.status, "cancelado");
    assert!(matches!(
        pix::cancel(&core, &ctx, &first.id).await,
        Err(CoreError::BusinessRule(_))
    ));
    // a venda continua a receber; uma nova cobrança fecha o ciclo
    assert_eq!(entry_status(&core, &s.id).await.0, "aberto");
    let second = pix::new_for_sale(&core, &ctx, &s.id, "simulador")
        .await
        .unwrap();
    assert_ne!(second.id, first.id);
    pix::check(&core, &ctx, &second.id).await.unwrap();
    let done = pix::check(&core, &ctx, &second.id).await.unwrap();
    assert_eq!(done.status, "pago");
    assert_eq!(bank_total(&core, &s.id).await, 1990);
    // recebida: não há o que cobrar
    assert!(matches!(
        pix::new_for_sale(&core, &ctx, &s.id, "simulador").await,
        Err(CoreError::BusinessRule(_))
    ));
}

#[tokio::test]
async fn refused_charge_shows_the_reason() {
    let (_dir, core, _) = onboarded().await;
    core.register_provider(Arc::new(Recusa));
    let ctx = manager(&core).await;
    let save = IntegrationSave {
        provider: "recusa".into(),
        environment: "sandbox".into(),
        enabled: true,
        config: Map::new(),
        secrets: Map::new(),
    };
    integrations::save(&core, &ctx, save).await.unwrap();
    let p = bread(&core).await;
    let s = sales::create(&ctx, sale(&p, "pix", Some("recusa")))
        .await
        .unwrap();
    let c = pix::for_sale(&core, &ctx, &s.id).await.unwrap().unwrap();
    let c = pix::check(&core, &ctx, &c.id).await.unwrap();
    assert_eq!(c.status, "erro");
    assert!(c.error.unwrap().contains("chave Pix não cadastrada"));
    // a venda segue a receber (pode ser recebida de outro jeito)
    assert_eq!(entry_status(&core, &s.id).await.0, "aberto");
}

#[tokio::test]
async fn manual_receipt_before_confirmation_is_not_doubled() {
    let (_dir, core, _) = onboarded().await;
    simulator(&core, "0").await;
    let p = bread(&core).await;
    let ctx = manager(&core).await;
    let s = sales::create(&ctx, sale(&p, "pix", Some("simulador")))
        .await
        .unwrap();
    let c = pix::for_sale(&core, &ctx, &s.id).await.unwrap().unwrap();
    pix::check(&core, &ctx, &c.id).await.unwrap();
    // alguém baixou o título à mão antes da consulta
    let entry: String = sqlx::query_scalar("SELECT id FROM fin_entries WHERE sale_id = ?")
        .bind(&s.id)
        .fetch_one(&core.pool())
        .await
        .unwrap();
    let fin = core.require(Module::Finance, Action::Manage).await.unwrap();
    nucleo_core::modules::finance::service::pay_entry(
        &fin,
        &entry,
        nucleo_core::modules::finance::models::PayInput {
            date: "2026-11-04".into(),
            amount: None,
            bank_account_id: None,
            payment_method: Some("pix".into()),
        },
    )
    .await
    .unwrap();
    let c = pix::check(&core, &ctx, &c.id).await.unwrap();
    assert_eq!(c.status, "pago");
    assert!(c.error.unwrap().contains("baixado à mão"));
    assert_eq!(bank_total(&core, &s.id).await, 1990);
}

#[tokio::test]
async fn cashier_can_sell_with_pix() {
    let (_dir, core, _) = onboarded().await;
    simulator(&core, "0").await;
    let p = bread(&core).await;
    create_member(&core, "caixa@padaria.com.br", "user").await;
    login_as(&core, "caixa@padaria.com.br").await;
    let ctx = core.require(Module::Sales, Action::Write).await.unwrap();
    // o caixa não configura integrações, mas vende com PIX
    assert!(core
        .require(Module::Integrations, Action::Read)
        .await
        .is_err());
    let s = sales::create(&ctx, sale(&p, "pix", Some("simulador")))
        .await
        .unwrap();
    let c = pix::for_sale(&core, &ctx, &s.id).await.unwrap().unwrap();
    pix::check(&core, &ctx, &c.id).await.unwrap();
    assert_eq!(pix::check(&core, &ctx, &c.id).await.unwrap().status, "pago");
}
