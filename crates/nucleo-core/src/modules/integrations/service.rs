use std::sync::Arc;

use serde_json::{Map, Value};

use crate::audit::{self, Entry};
use crate::db::{self, Conn, Installation};
use crate::error::{CoreError, CoreResult, Validator};
use crate::{Core, Ctx};

use super::catalog::{self, ProviderInfo};
use super::models::{
    CatalogItem, Integration, IntegrationOut, IntegrationSave, JobFilter, JobOut, QueueSummary,
    TickReport,
};
use super::provider::{Call, Outcome, Provider, ProviderError};
use super::repository::{self as repo, JobUpdate, NewJob};

/// Falhas seguidas antes de desistir e pedir ação do usuário.
pub const MAX_ATTEMPTS: i64 = 8;
/// Trabalhos por rodada do processador da fila.
const BATCH: i64 = 20;

const NOT_FOUND: &str = "Integração não encontrada.";

// ---------- tela ----------

fn to_out(core: &Core, i: &Integration) -> IntegrationOut {
    let opened = i.secrets.as_deref().map(|s| core.vault().open_sealed(s));
    let (secrets_set, secrets_readable) = match opened {
        None => (vec![], true),
        Some(None) => (vec![], false),
        Some(Some(bytes)) => {
            let keys = serde_json::from_slice::<Map<String, Value>>(&bytes)
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default();
            (keys, true)
        }
    };
    IntegrationOut {
        id: i.id.clone(),
        provider: i.provider.clone(),
        environment: i.environment.clone(),
        enabled: i.enabled,
        config: i.config.clone(),
        secrets_set,
        secrets_readable,
        status: i.status.clone(),
        status_detail: i.status_detail.clone(),
        checked_at: i.checked_at.clone(),
        updated_at: i.updated_at.clone(),
    }
}

/// Provedores disponíveis + planejados, cada um com a configuração atual (se houver).
pub async fn catalog(core: &Core, ctx: &Ctx) -> CoreResult<Vec<CatalogItem>> {
    let mut conn = ctx.db.acquire().await?;
    let saved = repo::list(&mut conn).await?;
    let mut infos: Vec<ProviderInfo> = core.providers().iter().map(|p| p.info()).collect();
    infos.sort_by_key(|i| i.name);
    for p in catalog::planned_infos() {
        if !infos.iter().any(|i| i.kind == p.kind) {
            infos.push(p);
        }
    }
    Ok(infos
        .into_iter()
        .map(|info| {
            let integration = saved
                .iter()
                .find(|i| i.provider == info.id)
                .map(|i| to_out(core, i));
            CatalogItem { info, integration }
        })
        .collect())
}

fn read_secrets(core: &Core, i: &Integration) -> Option<Map<String, Value>> {
    match &i.secrets {
        None => Some(Map::new()),
        Some(s) => core
            .vault()
            .open_sealed(s)
            .and_then(|b| serde_json::from_slice(&b).ok()),
    }
}

fn non_empty(v: Option<&Value>) -> bool {
    match v {
        Some(Value::String(s)) => !s.trim().is_empty(),
        Some(Value::Null) | None => false,
        Some(_) => true,
    }
}

/// Cria ou altera a configuração de um provedor. Segredos vazios mantêm os já guardados.
pub async fn save(core: &Core, ctx: &Ctx, data: IntegrationSave) -> CoreResult<IntegrationOut> {
    let provider = core.provider(&data.provider).ok_or_else(|| {
        CoreError::BusinessRule("Esta integração ainda não está disponível.".into())
    })?;
    let info = provider.info();

    let mut tx = ctx.db.begin().await?;
    let existing = repo::by_provider(&mut tx, &data.provider).await?;
    // segredos atuais (se ilegíveis, por ter vindo de outro computador, começam do zero)
    let mut secrets = existing
        .as_ref()
        .and_then(|i| read_secrets(core, i))
        .unwrap_or_default();

    let mut v = Validator::default();
    v.check(
        matches!(data.environment.as_str(), "sandbox" | "production"),
        "environment",
        "Escolha teste ou produção",
    );
    let mut config = Map::new();
    for f in info.fields.iter() {
        let key = f.key;
        if f.secret {
            if let Some(val) = data.secrets.get(key).filter(|v| non_empty(Some(v))) {
                let val = match val {
                    Value::String(s) => Value::String(s.trim().to_owned()),
                    other => other.clone(),
                };
                secrets.insert(key.to_owned(), val);
            }
            v.check(
                !f.required || non_empty(secrets.get(key)),
                &format!("secrets.{key}"),
                &format!("Informe {}", f.label),
            );
        } else {
            let val = data.config.get(key).cloned().unwrap_or(Value::Null);
            let val = match val {
                Value::String(s) => Value::String(s.trim().to_owned()),
                other => other,
            };
            v.check(
                !f.required || non_empty(Some(&val)),
                &format!("config.{key}"),
                &format!("Informe {}", f.label),
            );
            if non_empty(Some(&val)) {
                config.insert(key.to_owned(), val);
            }
        }
    }
    v.finish()?;

    let sealed = if secrets.is_empty() {
        None
    } else {
        Some(
            core.vault()
                .seal(Value::Object(secrets).to_string().as_bytes())?,
        )
    };
    let now = db::now();
    let after = match existing {
        Some(before) => {
            let changed = before.environment != data.environment
                || before.config != Value::Object(config.clone())
                || secrets_changed(&data.secrets);
            let mut after = before.clone();
            after.environment = data.environment;
            after.enabled = data.enabled;
            after.config = Value::Object(config);
            after.secrets = sealed;
            if changed {
                after.status = "unchecked".into();
                after.status_detail = None;
                after.checked_at = None;
            }
            after.updated_at = now;
            repo::update(&mut tx, &after).await?;
            audit::record_changes(
                &mut tx,
                &ctx.inst,
                Entry {
                    actor_id: Some(&ctx.user.id),
                    action: "integration.update",
                    entity: "integrations",
                    entity_id: Some(&after.id),
                    before: Some(audit_view(&before, false)),
                    after: Some(audit_view(&after, secrets_changed(&data.secrets))),
                },
            )
            .await?;
            after
        }
        None => {
            let created = Integration {
                id: db::new_id(),
                provider: data.provider,
                environment: data.environment,
                enabled: data.enabled,
                config: Value::Object(config),
                secrets: sealed,
                status: "unchecked".into(),
                status_detail: None,
                checked_at: None,
                created_at: now.clone(),
                updated_at: now,
            };
            repo::insert(&mut tx, &ctx.inst, &created).await?;
            audit::record(
                &mut tx,
                &ctx.inst,
                Entry {
                    actor_id: Some(&ctx.user.id),
                    action: "integration.create",
                    entity: "integrations",
                    entity_id: Some(&created.id),
                    before: None,
                    after: Some(audit_view(&created, secrets_changed(&data.secrets))),
                },
            )
            .await?;
            created
        }
    };
    tx.commit().await?;
    Ok(to_out(core, &after))
}

fn secrets_changed(input: &Map<String, Value>) -> bool {
    input.values().any(|v| non_empty(Some(v)))
}

/// Foto para a auditoria: nunca os segredos, só o aviso de que foram trocados.
fn audit_view(i: &Integration, secrets_changed: bool) -> Value {
    let mut v = audit::snapshot(i);
    if let Some(o) = v.as_object_mut() {
        o.remove("status");
        o.remove("status_detail");
        o.remove("checked_at");
        if secrets_changed {
            o.insert("credenciais".into(), Value::String("alteradas".into()));
        }
    }
    v
}

/// Testa as credenciais agora e guarda o resultado.
pub async fn test(core: &Core, ctx: &Ctx, provider_id: &str) -> CoreResult<IntegrationOut> {
    let provider = core
        .provider(provider_id)
        .ok_or_else(|| CoreError::NotFound(NOT_FOUND.into()))?;
    let mut conn = ctx.db.acquire().await?;
    let mut i = repo::by_provider(&mut conn, provider_id)
        .await?
        .ok_or_else(|| CoreError::NotFound(NOT_FOUND.into()))?;
    drop(conn);
    let secrets = read_secrets(core, &i).ok_or_else(|| {
        CoreError::BusinessRule(
            "As credenciais desta integração foram salvas em outro computador. Informe-as de novo."
                .into(),
        )
    })?;
    let secrets = Value::Object(secrets);
    let call = Call {
        http: core.http(),
        production: i.environment == "production",
        config: &i.config,
        secrets: &secrets,
    };
    let result = provider.check(&call).await;
    let now = db::now();
    match result {
        Ok(detail) => {
            i.status = "ok".into();
            i.status_detail = Some(detail);
        }
        Err(e) => {
            log::warn!("integração {provider_id}: teste falhou: {e:?}");
            i.status = "error".into();
            i.status_detail = Some(e.message());
        }
    }
    i.checked_at = Some(now.clone());
    i.updated_at = now;
    let mut conn = ctx.db.acquire().await?;
    repo::update(&mut conn, &i).await?;
    Ok(to_out(core, &i))
}

/// Remove a configuração (e as credenciais). Trabalhos pendentes são cancelados.
pub async fn remove(ctx: &Ctx, provider_id: &str) -> CoreResult<()> {
    let mut tx = ctx.db.begin().await?;
    let i = repo::by_provider(&mut tx, provider_id)
        .await?
        .ok_or_else(|| CoreError::NotFound(NOT_FOUND.into()))?;
    sqlx::query(
        "UPDATE integration_jobs SET status = 'canceled', updated_at = ?
         WHERE integration_id = ? AND status IN ('pending', 'failed')",
    )
    .bind(db::now())
    .bind(&i.id)
    .execute(&mut *tx)
    .await?;
    repo::soft_delete(&mut tx, &i.id).await?;
    audit::record(
        &mut tx,
        &ctx.inst,
        Entry {
            actor_id: Some(&ctx.user.id),
            action: "integration.delete",
            entity: "integrations",
            entity_id: Some(&i.id),
            before: Some(audit_view(&i, false)),
            after: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

// ---------- fila ----------

/// Põe um trabalho na fila, na transação de quem chama (ex.: a venda que gerou o PIX).
/// Funciona sem internet: o processador da fila envia quando houver conexão.
pub async fn enqueue(
    conn: &mut Conn,
    inst: &Installation,
    provider_id: &str,
    kind: &str,
    payload: &Value,
    reference: Option<(&str, &str)>,
) -> CoreResult<String> {
    let i = repo::by_provider(conn, provider_id)
        .await?
        .filter(|i| i.enabled)
        .ok_or_else(|| {
            CoreError::BusinessRule(
                "Esta integração não está ativada. Configure-a em Integrações.".into(),
            )
        })?;
    repo::insert_job(
        conn,
        inst,
        NewJob {
            integration_id: &i.id,
            kind,
            payload,
            ref_entity: reference.map(|r| r.0),
            ref_id: reference.map(|r| r.1),
        },
    )
    .await
}

pub async fn jobs(ctx: &Ctx, filter: JobFilter) -> CoreResult<Vec<JobOut>> {
    let mut conn = ctx.db.acquire().await?;
    let status = filter.status.filter(|s| !s.is_empty());
    repo::list_jobs(&mut conn, status.as_deref(), 300).await
}

pub async fn summary(ctx: &Ctx) -> CoreResult<QueueSummary> {
    let mut conn = ctx.db.acquire().await?;
    repo::summary(&mut conn).await
}

/// Trabalho que desistiu volta para a fila do zero.
pub async fn retry_job(ctx: &Ctx, id: &str) -> CoreResult<()> {
    let mut tx = ctx.db.begin().await?;
    let job = repo::get_job(&mut tx, id)
        .await?
        .ok_or_else(|| CoreError::NotFound("Item da fila não encontrado.".into()))?;
    if job.status != "failed" {
        return Err(CoreError::BusinessRule(
            "Só dá para tentar de novo o que falhou.".into(),
        ));
    }
    let now = db::now();
    repo::update_job(
        &mut tx,
        id,
        JobUpdate {
            status: "pending",
            attempts: 0,
            next_run_at: &now,
            last_error: job.last_error.as_deref(),
            result: None,
            finished_at: None,
        },
    )
    .await?;
    audit::record(
        &mut tx,
        &ctx.inst,
        Entry {
            actor_id: Some(&ctx.user.id),
            action: "integration.job_retry",
            entity: "integration_jobs",
            entity_id: Some(id),
            before: None,
            after: Some(serde_json::json!({ "kind": job.kind })),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn cancel_job(ctx: &Ctx, id: &str) -> CoreResult<()> {
    let mut tx = ctx.db.begin().await?;
    let job = repo::get_job(&mut tx, id)
        .await?
        .ok_or_else(|| CoreError::NotFound("Item da fila não encontrado.".into()))?;
    if !matches!(job.status.as_str(), "pending" | "failed") {
        return Err(CoreError::BusinessRule(
            "Este item já foi processado e não pode ser cancelado.".into(),
        ));
    }
    let now = db::now();
    repo::update_job(
        &mut tx,
        id,
        JobUpdate {
            status: "canceled",
            attempts: job.attempts,
            next_run_at: &job.next_run_at,
            last_error: job.last_error.as_deref(),
            result: None,
            finished_at: Some(&now),
        },
    )
    .await?;
    audit::record(
        &mut tx,
        &ctx.inst,
        Entry {
            actor_id: Some(&ctx.user.id),
            action: "integration.job_cancel",
            entity: "integration_jobs",
            entity_id: Some(id),
            before: None,
            after: Some(serde_json::json!({ "kind": job.kind })),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Espera antes da próxima tentativa: 30 s, 1 min, 2 min, … até 1 h.
pub fn backoff_secs(attempts: i64) -> i64 {
    let exp = attempts.clamp(1, 8) - 1;
    (30 * (1i64 << exp)).min(3600)
}

fn after_secs(secs: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::seconds(secs.max(1)))
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

/// Uma rodada do processador da fila. Roda em segundo plano (sem usuário logado)
/// e não segura transação durante a chamada à API.
pub async fn tick(core: &Core) -> CoreResult<TickReport> {
    let pool = core.pool();
    let inst = core.installation();
    let mut report = TickReport::default();
    let mut conn = pool.acquire().await?;
    let stale = (chrono::Utc::now() - chrono::Duration::minutes(10))
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    repo::release_stale(&mut conn, &stale).await?;
    let due = repo::due_jobs(&mut conn, &db::now(), BATCH).await?;
    drop(conn);

    for job in due {
        let mut conn = pool.acquire().await?;
        if !repo::claim(&mut conn, &job.id).await? {
            continue;
        }
        let integration = repo::get(&mut conn, &job.integration_id).await?;
        drop(conn);
        report.ran += 1;

        let outcome = match integration.as_ref().filter(|i| i.enabled) {
            None => Outcome::Error(ProviderError::Rejected(
                "a integração foi desativada ou removida".into(),
            )),
            Some(i) => match (core.provider(&i.provider), read_secrets(core, i)) {
                (None, _) => Outcome::Error(ProviderError::Rejected(
                    "provedor indisponível nesta versão".into(),
                )),
                (_, None) => Outcome::Error(ProviderError::Auth(
                    "credenciais salvas em outro computador".into(),
                )),
                (Some(p), Some(secrets)) => {
                    let secrets = Value::Object(secrets);
                    let call = Call {
                        http: core.http(),
                        production: i.environment == "production",
                        config: &i.config,
                        secrets: &secrets,
                    };
                    p.run(&call, &job).await
                }
            },
        };

        let mut tx = pool.begin().await?;
        let now = db::now();
        match outcome {
            Outcome::Done(result) => {
                if let Some(p) = integration
                    .as_ref()
                    .and_then(|i| core.provider(&i.provider))
                {
                    p.on_done(&mut tx, &inst, &job, &result).await?;
                }
                repo::update_job(
                    &mut tx,
                    &job.id,
                    JobUpdate {
                        status: "done",
                        attempts: 0,
                        next_run_at: &job.next_run_at,
                        last_error: None,
                        result: Some(&result),
                        finished_at: Some(&now),
                    },
                )
                .await?;
                report.done += 1;
            }
            Outcome::Wait { secs, note } => {
                repo::update_job(
                    &mut tx,
                    &job.id,
                    JobUpdate {
                        status: "pending",
                        attempts: 0,
                        next_run_at: &after_secs(secs),
                        last_error: note.as_deref(),
                        result: None,
                        finished_at: None,
                    },
                )
                .await?;
                report.waiting += 1;
            }
            Outcome::Error(e) => {
                log::warn!("fila {} ({}): {e:?}", job.kind, job.id);
                let attempts = job.attempts + 1;
                let give_up = !e.transient() || attempts >= MAX_ATTEMPTS;
                let message = e.message();
                repo::update_job(
                    &mut tx,
                    &job.id,
                    JobUpdate {
                        status: if give_up { "failed" } else { "pending" },
                        attempts,
                        next_run_at: &after_secs(backoff_secs(attempts)),
                        last_error: Some(&message),
                        result: None,
                        finished_at: give_up.then_some(now.as_str()),
                    },
                )
                .await?;
                if give_up {
                    report.failed += 1;
                } else {
                    report.retrying += 1;
                }
            }
        }
        tx.commit().await?;
    }
    Ok(report)
}

/// Registro de provedores do `Core`.
#[derive(Default)]
pub struct Registry {
    providers: Vec<Arc<dyn Provider>>,
}

impl Registry {
    pub fn register(&mut self, p: Arc<dyn Provider>) {
        let id = p.info().id;
        self.providers.retain(|x| x.info().id != id);
        self.providers.push(p);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.iter().find(|p| p.info().id == id).cloned()
    }

    pub fn all(&self) -> Vec<Arc<dyn Provider>> {
        self.providers.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps() {
        assert_eq!(backoff_secs(1), 30);
        assert_eq!(backoff_secs(2), 60);
        assert_eq!(backoff_secs(3), 120);
        assert_eq!(backoff_secs(8), 3600);
        assert_eq!(backoff_secs(50), 3600);
    }
}
