use std::path::Path;

use nucleo_core::modules::spreadsheet::models::{
    ImportPreview, ImportRequest, ImportResult, SheetInput,
};
use nucleo_core::modules::spreadsheet::service as sheet;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tauri::State;

use super::Res;

/// Exporta o que a tela mostra; qualquer pessoa logada exporta o que já pode ver.
#[tauri::command(rename_all = "snake_case")]
pub async fn export_xlsx(core: State<'_, Core>, path: String, sheets: Vec<SheetInput>) -> Res<()> {
    core.current().await?;
    sheet::export_xlsx(Path::new(&path), &sheets)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn import_preview(core: State<'_, Core>, path: String) -> Res<ImportPreview> {
    core.require(Module::Products, Action::Write).await?;
    sheet::preview(Path::new(&path))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn import_products(core: State<'_, Core>, data: ImportRequest) -> Res<ImportResult> {
    let ctx = core.require(Module::Products, Action::Write).await?;
    sheet::import_products(&ctx, data).await
}
