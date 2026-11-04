use std::path::{Path, PathBuf};

use nucleo_core::modules::backup::service::{self as backup, BackupInfo};
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tauri::State;

use super::Res;

#[tauri::command(rename_all = "snake_case")]
pub async fn backup_list(core: State<'_, Core>) -> Res<Vec<BackupInfo>> {
    core.require(Module::Backup, Action::Write).await?;
    backup::list(&core)
}

/// `dest`: caminho escolhido no "Salvar como" (sem ele, vai para a pasta de backups do app).
#[tauri::command(rename_all = "snake_case")]
pub async fn backup_create(core: State<'_, Core>, dest: Option<String>) -> Res<BackupInfo> {
    let ctx = core.require(Module::Backup, Action::Write).await?;
    backup::create(&core, &ctx, dest.map(PathBuf::from)).await
}

/// Só o dono. Encerra a sessão: o app volta para a tela de login.
#[tauri::command(rename_all = "snake_case")]
pub async fn backup_restore(core: State<'_, Core>, path: String) -> Res<BackupInfo> {
    let ctx = core.require(Module::Backup, Action::Manage).await?;
    backup::restore(&core, &ctx, Path::new(&path)).await
}
