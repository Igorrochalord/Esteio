//! Banco local: abertura do SQLite, migrações, ids e datas.

use std::path::Path;
use std::time::Duration;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{SqliteConnection, SqlitePool};

use crate::error::CoreResult;

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub type Conn = SqliteConnection;

/// Abre (criando se preciso) o arquivo do banco e aplica as migrações pendentes.
pub async fn open(path: &Path) -> CoreResult<SqlitePool> {
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// Maior versão de migração que este executável conhece.
pub fn latest_migration() -> i64 {
    MIGRATOR.iter().map(|m| m.version).max().unwrap_or(0)
}

/// UUID v7: ordenável pelo tempo e único entre instalações (pronto para sync).
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Agora em UTC, no mesmo formato do `strftime('%Y-%m-%dT%H:%M:%fZ')` do SQLite.
pub fn now() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

/// Identidade desta instalação (filial + dispositivo), gravada em toda linha de negócio.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Installation {
    pub device_id: String,
    pub branch_id: String,
}

pub async fn ensure_installation(pool: &SqlitePool) -> CoreResult<Installation> {
    sqlx::query(
        "INSERT INTO installation (id, device_id, branch_id, created_at) VALUES (1, ?, ?, ?)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(new_id())
    .bind(new_id())
    .bind(now())
    .execute(pool)
    .await?;
    Ok(
        sqlx::query_as("SELECT device_id, branch_id FROM installation WHERE id = 1")
            .fetch_one(pool)
            .await?,
    )
}
