use nucleo_core::modules::audit::models::{AuditEntry, AuditQuery, Page};
use nucleo_core::modules::audit::service as audit;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tauri::State;

use super::Res;

#[tauri::command(rename_all = "snake_case")]
pub async fn audit_list(core: State<'_, Core>, query: AuditQuery) -> Res<Page<AuditEntry>> {
    let ctx = core.require(Module::Audit, Action::Read).await?;
    audit::list(&ctx, query).await
}
