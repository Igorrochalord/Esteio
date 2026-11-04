use nucleo_core::modules::integrations::models::{
    CatalogItem, IntegrationOut, IntegrationSave, JobFilter, JobOut, QueueSummary, TickReport,
};
use nucleo_core::modules::integrations::service as integrations;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tauri::State;

use super::Res;

#[tauri::command(rename_all = "snake_case")]
pub async fn integrations_catalog(core: State<'_, Core>) -> Res<Vec<CatalogItem>> {
    let ctx = core.require(Module::Integrations, Action::Read).await?;
    integrations::catalog(&core, &ctx).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn integrations_save(
    core: State<'_, Core>,
    data: IntegrationSave,
) -> Res<IntegrationOut> {
    let ctx = core.require(Module::Integrations, Action::Manage).await?;
    integrations::save(&core, &ctx, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn integrations_test(core: State<'_, Core>, provider: String) -> Res<IntegrationOut> {
    let ctx = core.require(Module::Integrations, Action::Manage).await?;
    integrations::test(&core, &ctx, &provider).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn integrations_remove(core: State<'_, Core>, provider: String) -> Res<()> {
    let ctx = core.require(Module::Integrations, Action::Manage).await?;
    integrations::remove(&ctx, &provider).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn integrations_jobs(
    core: State<'_, Core>,
    filter: Option<JobFilter>,
) -> Res<Vec<JobOut>> {
    let ctx = core.require(Module::Integrations, Action::Read).await?;
    integrations::jobs(&ctx, filter.unwrap_or_default()).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn integrations_summary(core: State<'_, Core>) -> Res<QueueSummary> {
    let ctx = core.require(Module::Integrations, Action::Read).await?;
    integrations::summary(&ctx).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn integrations_job_retry(core: State<'_, Core>, id: String) -> Res<()> {
    let ctx = core.require(Module::Integrations, Action::Manage).await?;
    integrations::retry_job(&ctx, &id).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn integrations_job_cancel(core: State<'_, Core>, id: String) -> Res<()> {
    let ctx = core.require(Module::Integrations, Action::Manage).await?;
    integrations::cancel_job(&ctx, &id).await
}

/// "Enviar agora": roda a fila sem esperar o próximo ciclo.
#[tauri::command(rename_all = "snake_case")]
pub async fn integrations_run_now(core: State<'_, Core>) -> Res<TickReport> {
    core.require(Module::Integrations, Action::Manage).await?;
    integrations::tick(&core).await
}
