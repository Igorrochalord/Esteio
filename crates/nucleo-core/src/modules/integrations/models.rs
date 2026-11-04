use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Configuração de um provedor nesta instalação. `secrets` sai cifrado e nunca é serializado.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Integration {
    pub id: String,
    pub provider: String,
    pub environment: String,
    pub enabled: bool,
    #[sqlx(json)]
    pub config: Value,
    #[serde(skip_serializing)]
    pub secrets: Option<Vec<u8>>,
    pub status: String,
    pub status_detail: Option<String>,
    pub checked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// O que a tela vê: sem segredos, só se eles existem e se podem ser lidos neste computador.
#[derive(Debug, Clone, Serialize)]
pub struct IntegrationOut {
    pub id: String,
    pub provider: String,
    pub environment: String,
    pub enabled: bool,
    pub config: Value,
    /// chaves dos segredos já guardados (os valores nunca saem do núcleo)
    pub secrets_set: Vec<String>,
    /// `false` quando o banco veio de outro computador e as credenciais precisam ser redigitadas
    pub secrets_readable: bool,
    pub status: String,
    pub status_detail: Option<String>,
    pub checked_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct IntegrationSave {
    pub provider: String,
    pub environment: String,
    pub enabled: bool,
    #[serde(default)]
    pub config: Map<String, Value>,
    /// valor vazio ou ausente = mantém o que já estava guardado
    #[serde(default)]
    pub secrets: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Job {
    pub id: String,
    pub integration_id: String,
    pub kind: String,
    #[sqlx(json)]
    pub payload: Value,
    pub status: String,
    pub attempts: i64,
    pub next_run_at: String,
    pub last_error: Option<String>,
    #[sqlx(json(nullable))]
    pub result: Option<Value>,
    pub ref_entity: Option<String>,
    pub ref_id: Option<String>,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct JobOut {
    pub id: String,
    pub provider: String,
    pub kind: String,
    pub status: String,
    pub attempts: i64,
    pub next_run_at: String,
    pub last_error: Option<String>,
    pub ref_entity: Option<String>,
    pub ref_id: Option<String>,
    pub finished_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct JobFilter {
    /// `pending`, `running`, `done`, `failed`, `canceled` ou vazio = todos
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueSummary {
    pub pending: i64,
    pub running: i64,
    pub failed: i64,
    pub enabled_integrations: i64,
    pub last_success_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TickReport {
    pub ran: usize,
    pub done: usize,
    pub waiting: usize,
    pub retrying: usize,
    pub failed: usize,
}

/// Item da tela de integrações: o provedor e, se já configurado, a configuração.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogItem {
    #[serde(flatten)]
    pub info: super::catalog::ProviderInfo,
    pub integration: Option<IntegrationOut>,
}
