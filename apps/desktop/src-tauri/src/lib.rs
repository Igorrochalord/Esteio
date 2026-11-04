//! App desktop: a interface React chama os comandos daqui via `invoke`, e eles
//! delegam ao `nucleo-core`. Não há servidor; o banco é um arquivo local. As integrações
//! online (ADR 0003) só fazem chamadas de saída, por uma fila que tolera ficar sem internet.

mod commands;

use std::path::PathBuf;

use std::time::Duration;

use nucleo_core::modules::backup::service as backup;
use nucleo_core::modules::integrations::service as integrations;
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

/// Intervalo do processador da fila de integrações.
const QUEUE_INTERVAL: Duration = Duration::from_secs(15);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // só o núcleo abre o navegador (autorização das integrações); a interface não tem permissão
        .plugin(tauri_plugin_opener::init())
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

            // fila das integrações: sem internet, os trabalhos esperam e são reenviados depois
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(QUEUE_INTERVAL).await;
                    let core = handle.state::<Core>();
                    if let Err(e) = integrations::tick(&core).await {
                        log::error!("fila de integrações: {e:?}");
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::setup_status,
            commands::auth::login_branding,
            commands::auth::register,
            commands::auth::onboard,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::me,
            commands::auth::plans_catalog,
            commands::company::company_get,
            commands::company::company_update,
            commands::company::branding_update,
            commands::company::logo_upload,
            commands::company::logo_remove,
            commands::users::users_list,
            commands::users::users_create,
            commands::users::users_update,
            commands::users::users_approve,
            commands::users::users_reject,
            commands::users::users_reset_password,
            commands::audit::audit_list,
            commands::backup::backup_list,
            commands::backup::backup_create,
            commands::backup::backup_restore,
            commands::products::products_list,
            commands::products::products_create,
            commands::products::products_update,
            commands::products::stock_move,
            commands::products::stock_movements,
            commands::products::categories_list,
            commands::products::categories_create,
            commands::products::categories_rename,
            commands::products::categories_delete,
            commands::customers::customers_list,
            commands::customers::customers_create,
            commands::customers::customers_update,
            commands::sales::sales_list,
            commands::sales::sales_get,
            commands::sales::sales_create,
            commands::sales::sales_cancel,
            commands::sales::sellers_list,
            commands::service_orders::os_list,
            commands::service_orders::os_get,
            commands::service_orders::os_create,
            commands::service_orders::os_update,
            commands::service_orders::os_set_items,
            commands::service_orders::os_change_status,
            commands::service_orders::os_add_note,
            commands::service_orders::os_invoice,
            commands::service_orders::os_cancel,
            commands::reports::report_sales,
            commands::reports::report_products,
            commands::reports::report_stock,
            commands::reports::report_customers,
            commands::reports::report_sellers,
            commands::spreadsheet::export_xlsx,
            commands::spreadsheet::import_preview,
            commands::spreadsheet::import_products,
            commands::finance::fin_accounts,
            commands::finance::fin_account_create,
            commands::finance::fin_account_update,
            commands::finance::fin_categories,
            commands::finance::fin_category_create,
            commands::finance::fin_category_update,
            commands::finance::fin_entries,
            commands::finance::fin_entry_create,
            commands::finance::fin_entry_update,
            commands::finance::fin_entry_pay,
            commands::finance::fin_entry_unpay,
            commands::finance::fin_entry_cancel,
            commands::finance::fin_transactions,
            commands::finance::fin_transaction_create,
            commands::finance::fin_transfer,
            commands::finance::fin_transaction_delete,
            commands::finance::fin_ofx_preview,
            commands::finance::fin_ofx_import,
            commands::finance::fin_cash_flow,
            commands::finance::fin_dre,
            commands::finance::fin_summary,
            commands::integrations::integrations_catalog,
            commands::integrations::integrations_save,
            commands::integrations::integrations_test,
            commands::integrations::integrations_remove,
            commands::integrations::integrations_jobs,
            commands::integrations::integrations_summary,
            commands::integrations::integrations_job_retry,
            commands::integrations::integrations_job_cancel,
            commands::integrations::integrations_run_now,
            commands::integrations::integrations_authorize,
            commands::integrations::integrations_oauth_code,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Núcleo ERP");
}
