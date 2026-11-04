//! App desktop: a interface React chama os comandos daqui via `invoke`, e eles
//! delegam ao `nucleo-core`. Não há servidor nem HTTP; o banco é um arquivo local.

mod commands;

use std::path::PathBuf;

use nucleo_core::modules::backup::service as backup;
use nucleo_core::Core;
use tauri::Manager;

/// Pasta dos dados: `NUCLEO_DATA_DIR` (dev/seed) ou a pasta de dados do app no SO
/// (Windows: %APPDATA%\br.com.nucleoerp.desktop).
fn data_dir(app: &tauri::App) -> tauri::Result<PathBuf> {
    match std::env::var_os("NUCLEO_DATA_DIR") {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => app.path().app_data_dir(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = data_dir(app)?;
            let core = tauri::async_runtime::block_on(Core::open(&dir))?;
            app.manage(core);

            // backup automático diário, sem travar a abertura
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let core = handle.state::<Core>();
                if let Err(e) = backup::auto_daily(&core).await {
                    log::error!("backup automático falhou: {e:?}");
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::setup_status,
            commands::auth::onboard,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::me,
            commands::company::company_get,
            commands::company::company_update,
            commands::company::branding_update,
            commands::company::logo_upload,
            commands::company::logo_remove,
            commands::users::users_list,
            commands::users::users_create,
            commands::users::users_update,
            commands::audit::audit_list,
            commands::backup::backup_list,
            commands::backup::backup_create,
            commands::backup::backup_restore,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Núcleo ERP");
}
