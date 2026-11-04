//! Núcleo ERP: regras de negócio e banco local (SQLite). Sem GUI e sem HTTP; a camada
//! Tauri (`apps/desktop/src-tauri`) só valida a sessão/permissão e delega para cá.
//!
//! Padrão por módulo: `modules/<m>/{models,repository,service}.rs`.
//! Services recebem `&Ctx` (usuário logado já checado) ou `&Core` (login/onboarding),
//! devolvem `CoreError` e fazem `commit` da própria transação.

pub mod audit;
pub mod db;
pub mod error;
pub mod modules;
pub mod money;
pub mod permissions;
pub mod plans;
pub mod security;
pub mod validate;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};

use sqlx::SqlitePool;

use db::Installation;
pub use error::{CoreError, CoreResult};
use modules::users::{models::User, repository as users_repo};
use permissions::{Action, Module};

/// Estado do app: banco aberto + sessão do usuário (em memória, nunca em disco).
pub struct Core {
    data_dir: PathBuf,
    pool: RwLock<SqlitePool>,
    inst: RwLock<Installation>,
    session: Mutex<Option<String>>,
    /// serializa backup/restauração (troca do arquivo do banco)
    pub(crate) maintenance: tokio::sync::Mutex<()>,
}

/// Contexto de um comando autenticado: banco, instalação e usuário relido do banco.
pub struct Ctx {
    pub db: SqlitePool,
    pub inst: Installation,
    pub user: User,
}

fn read<T: Clone>(lock: &RwLock<T>) -> T {
    lock.read().unwrap_or_else(|e| e.into_inner()).clone()
}

fn write<T>(lock: &RwLock<T>, value: T) {
    *lock.write().unwrap_or_else(|e| e.into_inner()) = value;
}

impl Core {
    /// Abre (ou cria) o banco em `<data_dir>/nucleo.db` e aplica migrações.
    pub async fn open(data_dir: &Path) -> CoreResult<Self> {
        std::fs::create_dir_all(data_dir)?;
        let pool = db::open(&data_dir.join("nucleo.db")).await?;
        let inst = db::ensure_installation(&pool).await?;
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            pool: RwLock::new(pool),
            inst: RwLock::new(inst),
            session: Mutex::new(None),
            maintenance: tokio::sync::Mutex::new(()),
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("nucleo.db")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.data_dir.join("backups")
    }

    pub fn pool(&self) -> SqlitePool {
        read(&self.pool)
    }

    pub fn installation(&self) -> Installation {
        read(&self.inst)
    }

    pub(crate) fn replace_db(&self, pool: SqlitePool, inst: Installation) {
        write(&self.pool, pool);
        write(&self.inst, inst);
    }

    pub(crate) fn set_session(&self, user_id: Option<String>) {
        *self.session.lock().unwrap_or_else(|e| e.into_inner()) = user_id;
    }

    fn session_user_id(&self) -> Option<String> {
        self.session
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Usuário logado, relido do banco: desativar alguém corta o acesso na hora.
    pub async fn current(&self) -> CoreResult<Ctx> {
        let unauthorized =
            || CoreError::Unauthorized("Sua sessão terminou. Entre novamente.".into());
        let user_id = self.session_user_id().ok_or_else(unauthorized)?;
        let db = self.pool();
        let mut conn = db.acquire().await?;
        let user = users_repo::get(&mut conn, &user_id).await?;
        drop(conn);
        match user {
            Some(user) if user.is_active => Ok(Ctx {
                db,
                inst: self.installation(),
                user,
            }),
            _ => {
                self.set_session(None);
                Err(unauthorized())
            }
        }
    }

    /// `current()` + checagem da matriz de permissões.
    pub async fn require(&self, module: Module, action: Action) -> CoreResult<Ctx> {
        let ctx = self.current().await?;
        if !permissions::is_allowed(ctx.user.role(), module, action) {
            return Err(CoreError::Forbidden(
                "Você não tem permissão para fazer isso.".into(),
            ));
        }
        Ok(ctx)
    }
}
