use nucleo_core::modules::sales::models::{Sale, SaleInput, SaleQuery};
use nucleo_core::modules::sales::service as sales;
use nucleo_core::modules::users::models::SellerOption;
use nucleo_core::modules::users::service as users;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tauri::State;

use super::Res;

#[tauri::command(rename_all = "snake_case")]
pub async fn sales_list(core: State<'_, Core>, query: SaleQuery) -> Res<Vec<Sale>> {
    let ctx = core.require(Module::Sales, Action::Read).await?;
    sales::list(&ctx, query).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sales_get(core: State<'_, Core>, id: String) -> Res<Sale> {
    let ctx = core.require(Module::Sales, Action::Read).await?;
    sales::get(&ctx, &id).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sales_create(core: State<'_, Core>, data: SaleInput) -> Res<Sale> {
    let ctx = core.require(Module::Sales, Action::Write).await?;
    sales::create(&ctx, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sales_cancel(core: State<'_, Core>, id: String, reason: String) -> Res<Sale> {
    let ctx = core.require(Module::Sales, Action::Manage).await?;
    sales::cancel(&ctx, &id, &reason).await
}

/// Vendedores para o formulário de venda.
#[tauri::command(rename_all = "snake_case")]
pub async fn sellers_list(core: State<'_, Core>) -> Res<Vec<SellerOption>> {
    let ctx = core.require(Module::Sales, Action::Write).await?;
    users::sellers(&ctx).await
}
