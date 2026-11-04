use nucleo_core::modules::service_orders::models::{
    InvoiceInput, OsInput, OsItemInput, OsPatch, OsQuery, ServiceOrder,
};
use nucleo_core::modules::service_orders::service as os;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tauri::State;

use super::Res;

#[tauri::command(rename_all = "snake_case")]
pub async fn os_list(core: State<'_, Core>, query: OsQuery) -> Res<Vec<ServiceOrder>> {
    let ctx = core.require(Module::ServiceOrders, Action::Read).await?;
    os::list(&ctx, query).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn os_get(core: State<'_, Core>, id: String) -> Res<ServiceOrder> {
    let ctx = core.require(Module::ServiceOrders, Action::Read).await?;
    os::get(&ctx, &id).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn os_create(core: State<'_, Core>, data: OsInput) -> Res<ServiceOrder> {
    let ctx = core.require(Module::ServiceOrders, Action::Write).await?;
    os::create(&ctx, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn os_update(core: State<'_, Core>, id: String, data: OsPatch) -> Res<ServiceOrder> {
    let ctx = core.require(Module::ServiceOrders, Action::Write).await?;
    os::update(&ctx, &id, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn os_set_items(
    core: State<'_, Core>,
    id: String,
    items: Vec<OsItemInput>,
) -> Res<ServiceOrder> {
    let ctx = core.require(Module::ServiceOrders, Action::Write).await?;
    os::set_items(&ctx, &id, items).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn os_change_status(
    core: State<'_, Core>,
    id: String,
    status: String,
    note: Option<String>,
) -> Res<ServiceOrder> {
    let ctx = core.require(Module::ServiceOrders, Action::Write).await?;
    os::change_status(&ctx, &id, &status, note).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn os_add_note(core: State<'_, Core>, id: String, note: String) -> Res<ServiceOrder> {
    let ctx = core.require(Module::ServiceOrders, Action::Write).await?;
    os::add_note(&ctx, &id, &note).await
}

/// Faturar gera uma venda: exige também permissão de registrar vendas.
#[tauri::command(rename_all = "snake_case")]
pub async fn os_invoice(
    core: State<'_, Core>,
    id: String,
    data: InvoiceInput,
) -> Res<ServiceOrder> {
    core.require(Module::Sales, Action::Write).await?;
    let ctx = core.require(Module::ServiceOrders, Action::Write).await?;
    os::invoice(&ctx, &id, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn os_cancel(core: State<'_, Core>, id: String, reason: String) -> Res<ServiceOrder> {
    let ctx = core.require(Module::ServiceOrders, Action::Manage).await?;
    os::cancel(&ctx, &id, &reason).await
}
