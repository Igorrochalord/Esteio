use nucleo_core::modules::products::models::{
    Category, Movement, MovementQuery, Product, ProductInput, ProductPatch, StockInput,
};
use nucleo_core::modules::products::service as products;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tauri::State;

use super::Res;

#[tauri::command(rename_all = "snake_case")]
pub async fn products_list(core: State<'_, Core>) -> Res<Vec<Product>> {
    let ctx = core.require(Module::Products, Action::Read).await?;
    products::list(&ctx).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn products_create(core: State<'_, Core>, data: ProductInput) -> Res<Product> {
    let ctx = core.require(Module::Products, Action::Write).await?;
    products::create(&ctx, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn products_update(
    core: State<'_, Core>,
    id: String,
    data: ProductPatch,
) -> Res<Product> {
    let ctx = core.require(Module::Products, Action::Write).await?;
    products::update(&ctx, &id, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn stock_move(core: State<'_, Core>, data: StockInput) -> Res<Product> {
    let ctx = core.require(Module::Products, Action::Write).await?;
    products::move_stock(&ctx, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn stock_movements(core: State<'_, Core>, query: MovementQuery) -> Res<Vec<Movement>> {
    let ctx = core.require(Module::Products, Action::Read).await?;
    products::movements(&ctx, query).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn categories_list(core: State<'_, Core>) -> Res<Vec<Category>> {
    let ctx = core.require(Module::Products, Action::Read).await?;
    products::categories(&ctx).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn categories_create(core: State<'_, Core>, name: String) -> Res<Category> {
    let ctx = core.require(Module::Products, Action::Write).await?;
    products::create_category(&ctx, &name).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn categories_rename(core: State<'_, Core>, id: String, name: String) -> Res<()> {
    let ctx = core.require(Module::Products, Action::Write).await?;
    products::rename_category(&ctx, &id, &name).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn categories_delete(core: State<'_, Core>, id: String) -> Res<()> {
    let ctx = core.require(Module::Products, Action::Write).await?;
    products::delete_category(&ctx, &id).await
}
