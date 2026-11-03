//! Cópias de segurança do banco. Como os dados só existem neste computador, perder o
//! arquivo é o maior risco do sistema: há backup automático diário (mantém 7) e manual,
//! e a restauração sempre guarda antes uma cópia do estado atual.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::json;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection, SqlitePool};

use crate::audit::{self, Entry};
use crate::db;
use crate::error::{CoreError, CoreResult};
use crate::{Core, Ctx};

pub const KEEP_AUTO: usize = 7;
const AUTO_EVERY: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub created_at: String,
    /// "auto", "manual" ou "pre-restore"
    pub kind: String,
}

fn stamp() -> String {
    chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f").to_string()
}

fn kind_of(name: &str) -> &'static str {
    if name.starts_with("auto-") {
        "auto"
    } else if name.starts_with("pre-restore-") {
        "pre-restore"
    } else {
        "manual"
    }
}

fn info(path: &Path) -> CoreResult<BackupInfo> {
    let meta = std::fs::metadata(path)?;
    let modified: chrono::DateTime<chrono::Utc> = meta.modified()?.into();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(BackupInfo {
        kind: kind_of(&name).into(),
        name,
        path: path.to_string_lossy().into_owned(),
        size_bytes: meta.len(),
        created_at: modified.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
    })
}

/// Cópia consistente do banco aberto (funciona com o app em uso).
async fn vacuum_into(pool: &SqlitePool, dest: &Path) -> CoreResult<BackupInfo> {
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if dest.exists() {
        std::fs::remove_file(dest)?; // o diálogo de "salvar como" já confirmou a substituição
    }
    sqlx::query("VACUUM INTO ?")
        .bind(dest.to_string_lossy().as_ref())
        .execute(pool)
        .await?;
    info(dest)
}

async fn audit_backup(
    core: &Core,
    actor: Option<&str>,
    action: &str,
    after: serde_json::Value,
) -> CoreResult<()> {
    let mut conn = core.pool().acquire().await?;
    let entry = Entry {
        actor_id: actor,
        action,
        entity: "backup",
        entity_id: None,
        before: None,
        after: Some(after),
    };
    audit::record(&mut conn, &core.installation(), entry).await
}

/// Backup manual. Sem `dest`, vai para a pasta de backups do app.
pub async fn create(core: &Core, ctx: &Ctx, dest: Option<PathBuf>) -> CoreResult<BackupInfo> {
    let _guard = core.maintenance.lock().await;
    let dest = dest.unwrap_or_else(|| core.backups_dir().join(format!("manual-{}.db", stamp())));
    let created = vacuum_into(&core.pool(), &dest).await?;
    audit_backup(
        core,
        Some(&ctx.user.id),
        "backup.create",
        json!({ "file": created.name }),
    )
    .await?;
    Ok(created)
}

/// Backups da pasta do app, do mais novo para o mais antigo.
pub fn list(core: &Core) -> CoreResult<Vec<BackupInfo>> {
    let dir = core.backups_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut items = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "db"))
        .map(|p| info(&p))
        .collect::<CoreResult<Vec<_>>>()?;
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.name.cmp(&a.name)));
    Ok(items)
}

/// Chamado na abertura do app: um backup automático a cada 24 h, mantendo os 7 últimos.
pub async fn auto_daily(core: &Core) -> CoreResult<Option<BackupInfo>> {
    let _guard = core.maintenance.lock().await;
    let autos: Vec<BackupInfo> = list(core)?
        .into_iter()
        .filter(|b| b.kind == "auto")
        .collect();
    let recent = autos
        .first()
        .and_then(|b| std::fs::metadata(&b.path).ok())
        .and_then(|m| m.modified().ok());
    if recent.is_some_and(|t| t.elapsed().unwrap_or_default() < AUTO_EVERY) {
        return Ok(None);
    }
    let dest = core.backups_dir().join(format!("auto-{}.db", stamp()));
    let created = vacuum_into(&core.pool(), &dest).await?;
    for old in autos.iter().skip(KEEP_AUTO - 1) {
        let _ = std::fs::remove_file(&old.path);
    }
    Ok(Some(created))
}

fn invalid_backup() -> CoreError {
    CoreError::BusinessRule("Este arquivo não é um backup válido do Núcleo ERP".into())
}

/// Confere se o arquivo é um banco íntegro deste sistema e de versão compatível.
async fn inspect(path: &Path) -> CoreResult<()> {
    let opts = SqliteConnectOptions::new().filename(path).read_only(true);
    let mut conn = SqliteConnection::connect_with(&opts)
        .await
        .map_err(|_| invalid_backup())?;
    let result = async {
        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&mut conn)
            .await?;
        let version: Option<i64> = sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations")
            .fetch_one(&mut conn)
            .await?;
        let _: i64 = sqlx::query_scalar("SELECT count(*) FROM company")
            .fetch_one(&mut conn)
            .await?;
        Ok::<_, sqlx::Error>((integrity, version))
    }
    .await;
    let _ = conn.close().await;
    let (integrity, version) = result.map_err(|_| invalid_backup())?;
    if integrity != "ok" {
        return Err(invalid_backup());
    }
    if version.unwrap_or(0) > db::latest_migration() {
        return Err(CoreError::BusinessRule(
            "Este backup é de uma versão mais nova do Núcleo ERP. Atualize o programa antes de restaurar.".into(),
        ));
    }
    Ok(())
}

fn remove_db_files(db: &Path) -> std::io::Result<()> {
    for suffix in ["", "-wal", "-shm"] {
        let p = PathBuf::from(format!("{}{suffix}", db.display()));
        if p.exists() {
            std::fs::remove_file(p)?;
        }
    }
    Ok(())
}

/// Substitui o banco atual pelo backup. Guarda antes uma cópia do estado atual
/// (`pre-restore-*.db`) e encerra a sessão: os usuários podem ser outros.
pub async fn restore(core: &Core, ctx: &Ctx, source: &Path) -> CoreResult<BackupInfo> {
    let _guard = core.maintenance.lock().await;
    let db_path = core.db_path();
    let staging = core.data_dir().join("nucleo.db.restore");
    remove_db_files(&staging)?;
    std::fs::copy(source, &staging).map_err(|_| invalid_backup())?;
    if let Err(e) = inspect(&staging).await {
        let _ = remove_db_files(&staging);
        return Err(e);
    }

    let safety = vacuum_into(
        &core.pool(),
        &core
            .backups_dir()
            .join(format!("pre-restore-{}.db", stamp())),
    )
    .await?;
    core.pool().close().await;
    remove_db_files(&db_path)?;
    std::fs::rename(&staging, &db_path)?;

    let pool = match db::open(&db_path).await {
        Ok(pool) => pool,
        Err(e) => {
            // volta ao estado anterior
            log::error!("restauração falhou ao abrir o banco: {e:?}");
            remove_db_files(&db_path)?;
            std::fs::copy(&safety.path, &db_path)?;
            let pool = db::open(&db_path).await?;
            let inst = db::ensure_installation(&pool).await?;
            core.replace_db(pool, inst);
            return Err(invalid_backup());
        }
    };
    let inst = db::ensure_installation(&pool).await?;
    core.replace_db(pool, inst);
    core.set_session(None);

    let source_name = source.file_name().map(|n| n.to_string_lossy().into_owned());
    audit_backup(
        core,
        Some(&ctx.user.id),
        "backup.restore",
        json!({ "file": source_name, "safety_copy": safety.name }),
    )
    .await?;
    Ok(safety)
}
