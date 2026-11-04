use nucleo_core::modules::reports::models::{
    CustomersReport, ProductsReport, ReportQuery, SalesSummary, SellerRow, StockReport,
};
use nucleo_core::modules::reports::service as reports;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tauri::State;

use super::Res;

#[tauri::command(rename_all = "snake_case")]
pub async fn report_sales(core: State<'_, Core>, query: ReportQuery) -> Res<SalesSummary> {
    let ctx = core.require(Module::Reports, Action::Read).await?;
    reports::sales_summary(&ctx, query).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn report_products(core: State<'_, Core>, query: ReportQuery) -> Res<ProductsReport> {
    let ctx = core.require(Module::Reports, Action::Read).await?;
    reports::products(&ctx, query).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn report_stock(core: State<'_, Core>, query: ReportQuery) -> Res<StockReport> {
    let ctx = core.require(Module::Reports, Action::Read).await?;
    reports::stock(&ctx, query).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn report_customers(core: State<'_, Core>, query: ReportQuery) -> Res<CustomersReport> {
    let ctx = core.require(Module::Reports, Action::Read).await?;
    reports::customers(&ctx, query).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn report_sellers(core: State<'_, Core>, query: ReportQuery) -> Res<Vec<SellerRow>> {
    let ctx = core.require(Module::Reports, Action::Read).await?;
    reports::sellers(&ctx, query).await
}
