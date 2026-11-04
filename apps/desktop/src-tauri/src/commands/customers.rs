use nucleo_core::modules::customers::models::{Customer, CustomerInput, CustomerPatch};
use nucleo_core::modules::customers::service as customers;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tauri::State;

use super::Res;

#[tauri::command(rename_all = "snake_case")]
pub async fn customers_list(core: State<'_, Core>) -> Res<Vec<Customer>> {
    let ctx = core.require(Module::Customers, Action::Read).await?;
    customers::list(&ctx).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn customers_create(core: State<'_, Core>, data: CustomerInput) -> Res<Customer> {
    let ctx = core.require(Module::Customers, Action::Write).await?;
    customers::create(&ctx, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn customers_update(
    core: State<'_, Core>,
    id: String,
    data: CustomerPatch,
) -> Res<Customer> {
    let ctx = core.require(Module::Customers, Action::Write).await?;
    customers::update(&ctx, &id, data).await
}
