use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, sqlx::FromRow)]
pub struct AuditRow {
    pub id: String,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub id: String,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<String>,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub created_at: String,
}

impl From<AuditRow> for AuditEntry {
    fn from(r: AuditRow) -> Self {
        let parse = |s: Option<String>| s.and_then(|s| serde_json::from_str(&s).ok());
        Self {
            id: r.id,
            user_id: r.user_id,
            user_name: r.user_name,
            action: r.action,
            entity: r.entity,
            entity_id: r.entity_id,
            before: parse(r.before),
            after: parse(r.after),
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub entity: Option<String>,
    pub entity_id: Option<String>,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
