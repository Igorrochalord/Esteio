//! Integrações: configuração com credenciais cifradas, teste de conexão, fila com
//! novas tentativas, permissões e auditoria. Usa um provedor falso (sem internet).

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use common::*;
use nucleo_core::db::{Conn, Installation};
use nucleo_core::modules::integrations::catalog::{Field, Kind, ProviderInfo};
use nucleo_core::modules::integrations::models::{IntegrationSave, Job, JobFilter};
use nucleo_core::modules::integrations::provider::{Call, Outcome, Provider, ProviderError};
use nucleo_core::modules::integrations::service as integrations;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::{Core, CoreError, CoreResult};
use serde_json::{json, Map, Value};

#[derive(Default)]
struct Fake {
    applied: AtomicUsize,
}

#[async_trait]
impl Provider for Fake {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "teste",
            name: "Provedor de teste",
            kind: Kind::Other,
            description: "usado nos testes",
            fields: vec![
                Field {
                    key: "conta",
                    label: "a conta",
                    secret: false,
                    required: true,
                    help: None,
                },
                Field {
                    key: "token",
                    label: "o token",
                    secret: true,
                    required: true,
                    help: None,
                },
            ],
            available: true,
            phase: None,
        }
    }

    async fn check(&self, call: &Call<'_>) -> Result<String, ProviderError> {
        match call.secret("token") {
            Some("bom") => Ok(format!(
                "Conta: {}",
                call.config_str("conta").unwrap_or("?")
            )),
            _ => Err(ProviderError::Auth("401".into())),
        }
    }

    async fn run(&self, _call: &Call<'_>, job: &Job) -> Outcome {
        match job.kind.as_str() {
            "ok" => Outcome::Done(json!({ "pago": true })),
            "wait" => Outcome::Wait {
                secs: 60,
                note: Some("aguardando pagamento".into()),
            },
            "offline" => Outcome::Error(ProviderError::Offline),
            _ => Outcome::Error(ProviderError::Rejected("dados inválidos".into())),
        }
    }

    async fn on_done(
        &self,
        _conn: &mut Conn,
        _inst: &Installation,
        _job: &Job,
        result: &Value,
    ) -> CoreResult<()> {
        assert_eq!(result["pago"], true);
        self.applied.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn input(token: &str) -> IntegrationSave {
    let mut config = Map::new();
    config.insert("conta".into(), json!("Loja 1"));
    let mut secrets = Map::new();
    secrets.insert("token".into(), json!(token));
    IntegrationSave {
        provider: "teste".into(),
        environment: "sandbox".into(),
        enabled: true,
        config,
        secrets,
    }
}

async fn configured() -> (tempfile::TempDir, Core, Arc<Fake>) {
    let (dir, core, _) = onboarded().await;
    let fake = Arc::new(Fake::default());
    core.register_provider(fake.clone());
    let ctx = core
        .require(Module::Integrations, Action::Manage)
        .await
        .unwrap();
    integrations::save(&core, &ctx, input("bom")).await.unwrap();
    (dir, core, fake)
}

async fn enqueue(core: &Core, kind: &str) -> String {
    let pool = core.pool();
    let mut tx = pool.begin().await.unwrap();
    let id = integrations::enqueue(
        &mut tx,
        &core.installation(),
        "teste",
        kind,
        &json!({}),
        Some(("sales", "venda-1")),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    id
}

async fn job_row(core: &Core, id: &str) -> (String, i64, Option<String>) {
    sqlx::query_as("SELECT status, attempts, last_error FROM integration_jobs WHERE id = ?")
        .bind(id)
        .fetch_one(&core.pool())
        .await
        .unwrap()
}

/// Faz o trabalho vencer agora (simula a passagem do tempo).
async fn make_due(core: &Core, id: &str) {
    sqlx::query(
        "UPDATE integration_jobs SET next_run_at = '2000-01-01T00:00:00.000Z' WHERE id = ?",
    )
    .bind(id)
    .execute(&core.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn catalog_lists_available_and_planned() {
    let (_dir, core, _) = configured().await;
    let ctx = core
        .require(Module::Integrations, Action::Read)
        .await
        .unwrap();
    let items = integrations::catalog(&core, &ctx).await.unwrap();
    let teste = items.iter().find(|i| i.info.id == "teste").unwrap();
    assert!(teste.info.available);
    let saved = teste.integration.as_ref().unwrap();
    assert_eq!(saved.secrets_set, vec!["token".to_string()]);
    assert!(saved.secrets_readable);
    assert_eq!(saved.config["conta"], "Loja 1");
    // planejados aparecem como "em breve"
    let pix = items.iter().find(|i| i.info.id == "pix").unwrap();
    assert!(!pix.info.available);
    assert_eq!(pix.info.phase, Some("6b"));
}

#[tokio::test]
async fn validation_and_unavailable_provider() {
    let (_dir, core, _) = onboarded().await;
    core.register_provider(Arc::new(Fake::default()));
    let ctx = core
        .require(Module::Integrations, Action::Manage)
        .await
        .unwrap();

    let mut empty = input("");
    empty.config.clear();
    empty.environment = "prod".into();
    let Err(CoreError::Validation(f)) = integrations::save(&core, &ctx, empty).await else {
        panic!("esperava validação");
    };
    assert!(f.contains_key("config.conta"));
    assert!(f.contains_key("secrets.token"));
    assert!(f.contains_key("environment"));

    let mut pix = input("x");
    pix.provider = "pix".into();
    assert!(matches!(
        integrations::save(&core, &ctx, pix).await,
        Err(CoreError::BusinessRule(_))
    ));
}

#[tokio::test]
async fn secrets_are_encrypted_kept_and_never_audited() {
    let (_dir, core, _) = configured().await;
    let raw: Vec<u8> = sqlx::query_scalar("SELECT secrets FROM integrations")
        .fetch_one(&core.pool())
        .await
        .unwrap();
    assert!(!String::from_utf8_lossy(&raw).contains("bom"));

    // salvar de novo sem informar o token mantém o anterior
    let ctx = core
        .require(Module::Integrations, Action::Manage)
        .await
        .unwrap();
    let mut again = input("");
    again.config.insert("conta".into(), json!("Loja 2"));
    integrations::save(&core, &ctx, again).await.unwrap();
    let out = integrations::test(&core, &ctx, "teste").await.unwrap();
    assert_eq!(out.status, "ok");
    assert_eq!(out.status_detail.as_deref(), Some("Conta: Loja 2"));

    let audit: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT action, after FROM audit_logs WHERE entity = 'integrations' ORDER BY created_at",
    )
    .fetch_all(&core.pool())
    .await
    .unwrap();
    assert_eq!(audit[0].0, "integration.create");
    assert_eq!(audit[1].0, "integration.update");
    for (_, after) in &audit {
        let after = after.as_deref().unwrap_or("");
        assert!(
            !after.contains("bom") && !after.contains("secrets"),
            "{after}"
        );
    }
    assert!(audit[0].1.as_deref().unwrap().contains("alteradas"));
}

#[tokio::test]
async fn test_connection_reports_refused_credentials() {
    let (_dir, core, _) = configured().await;
    let ctx = core
        .require(Module::Integrations, Action::Manage)
        .await
        .unwrap();
    integrations::save(&core, &ctx, input("ruim"))
        .await
        .unwrap();
    let out = integrations::test(&core, &ctx, "teste").await.unwrap();
    assert_eq!(out.status, "error");
    assert!(out.status_detail.unwrap().contains("credenciais"));
}

#[tokio::test]
async fn credentials_from_another_computer_must_be_reentered() {
    let (dir, core, _) = configured().await;
    drop(core);
    // outro computador = outra chave (o banco veio por backup, a chave não)
    std::fs::remove_file(dir.path().join("integracoes.key")).unwrap();
    let core = Core::open(dir.path()).await.unwrap();
    core.register_provider(Arc::new(Fake::default()));
    login_as(&core, OWNER_EMAIL).await;
    let ctx = core
        .require(Module::Integrations, Action::Manage)
        .await
        .unwrap();
    let items = integrations::catalog(&core, &ctx).await.unwrap();
    let saved = items
        .iter()
        .find(|i| i.info.id == "teste")
        .and_then(|i| i.integration.clone())
        .unwrap();
    assert!(!saved.secrets_readable);
    assert!(matches!(
        integrations::test(&core, &ctx, "teste").await,
        Err(CoreError::BusinessRule(_))
    ));
    // redigitar resolve
    integrations::save(&core, &ctx, input("bom")).await.unwrap();
    assert_eq!(
        integrations::test(&core, &ctx, "teste")
            .await
            .unwrap()
            .status,
        "ok"
    );
}

#[tokio::test]
async fn queue_runs_done_wait_retry_and_fail() {
    let (_dir, core, fake) = configured().await;
    let ok = enqueue(&core, "ok").await;
    let wait = enqueue(&core, "wait").await;
    let offline = enqueue(&core, "offline").await;
    let bad = enqueue(&core, "bad").await;

    let r = integrations::tick(&core).await.unwrap();
    assert_eq!(
        (r.ran, r.done, r.waiting, r.retrying, r.failed),
        (4, 1, 1, 1, 1)
    );
    assert_eq!(fake.applied.load(Ordering::SeqCst), 1);

    assert_eq!(job_row(&core, &ok).await.0, "done");
    let (st, attempts, note) = job_row(&core, &wait).await;
    assert_eq!((st.as_str(), attempts), ("pending", 0));
    assert_eq!(note.as_deref(), Some("aguardando pagamento"));
    let (st, attempts, err) = job_row(&core, &offline).await;
    assert_eq!((st.as_str(), attempts), ("pending", 1));
    assert!(err.unwrap().contains("internet"));
    assert_eq!(job_row(&core, &bad).await.0, "failed");

    // nada vencido: a rodada seguinte não chama a API
    assert_eq!(integrations::tick(&core).await.unwrap().ran, 0);

    // sem internet por muito tempo: desiste depois de MAX_ATTEMPTS
    for _ in 1..integrations::MAX_ATTEMPTS {
        make_due(&core, &offline).await;
        integrations::tick(&core).await.unwrap();
    }
    let (st, attempts, _) = job_row(&core, &offline).await;
    assert_eq!(
        (st.as_str(), attempts),
        ("failed", integrations::MAX_ATTEMPTS)
    );

    let ctx = core
        .require(Module::Integrations, Action::Manage)
        .await
        .unwrap();
    let summary = integrations::summary(&ctx).await.unwrap();
    assert_eq!((summary.pending, summary.failed), (1, 2));
    assert!(summary.last_success_at.is_some());

    // tentar de novo / cancelar
    integrations::retry_job(&ctx, &offline).await.unwrap();
    let (st, attempts, _) = job_row(&core, &offline).await;
    assert_eq!((st.as_str(), attempts), ("pending", 0));
    assert!(matches!(
        integrations::retry_job(&ctx, &ok).await,
        Err(CoreError::BusinessRule(_))
    ));
    integrations::cancel_job(&ctx, &bad).await.unwrap();
    assert_eq!(job_row(&core, &bad).await.0, "canceled");
    let failed = integrations::jobs(
        &ctx,
        JobFilter {
            status: Some("failed".into()),
        },
    )
    .await
    .unwrap();
    assert!(failed.is_empty());
}

#[tokio::test]
async fn enqueue_requires_enabled_integration_and_remove_cancels_queue() {
    let (_dir, core, _) = configured().await;
    let pending = enqueue(&core, "wait").await;
    let ctx = core
        .require(Module::Integrations, Action::Manage)
        .await
        .unwrap();
    let mut off = input("");
    off.enabled = false;
    integrations::save(&core, &ctx, off).await.unwrap();

    let pool = core.pool();
    let mut tx = pool.begin().await.unwrap();
    let err = integrations::enqueue(
        &mut tx,
        &core.installation(),
        "teste",
        "ok",
        &json!({}),
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, CoreError::BusinessRule(_)));
    drop(tx);

    // desativada: o que já estava na fila falha sem chamar a API
    integrations::tick(&core).await.unwrap();
    assert_eq!(job_row(&core, &pending).await.0, "failed");

    integrations::remove(&ctx, "teste").await.unwrap();
    let items = integrations::catalog(&core, &ctx).await.unwrap();
    assert!(items
        .iter()
        .find(|i| i.info.id == "teste")
        .unwrap()
        .integration
        .is_none());
    assert_eq!(job_row(&core, &pending).await.0, "canceled");
}

#[tokio::test]
async fn permissions_by_role() {
    let (_dir, core, _) = configured().await;
    create_member(&core, "admin@padaria.com.br", "admin").await;
    create_member(&core, "caixa@padaria.com.br", "user").await;

    login_as(&core, "admin@padaria.com.br").await;
    assert!(core
        .require(Module::Integrations, Action::Manage)
        .await
        .is_ok());

    login_as(&core, "caixa@padaria.com.br").await;
    assert!(matches!(
        core.require(Module::Integrations, Action::Read).await,
        Err(CoreError::Forbidden(_))
    ));
}
