use nucleo_core::modules::company::models::{Branding, BrandingUpdate, CompanyOut, CompanyUpdate};
use nucleo_core::modules::company::service as company;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tauri::State;

use super::Res;

#[tauri::command(rename_all = "snake_case")]
pub async fn company_get(core: State<'_, Core>) -> Res<CompanyOut> {
    let ctx = core.require(Module::Settings, Action::Read).await?;
    company::get(&ctx).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn company_update(core: State<'_, Core>, data: CompanyUpdate) -> Res<CompanyOut> {
    let ctx = core.require(Module::Settings, Action::Manage).await?;
    company::update(&ctx, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn branding_update(core: State<'_, Core>, data: BrandingUpdate) -> Res<Branding> {
    let ctx = core.require(Module::Settings, Action::Manage).await?;
    company::update_branding(&ctx, data).await
}

/// `content`: imagem em base64 ou data URL (PNG/JPEG, até 2 MB).
#[tauri::command(rename_all = "snake_case")]
pub async fn logo_upload(core: State<'_, Core>, content: String) -> Res<Branding> {
    let ctx = core.require(Module::Settings, Action::Manage).await?;
    company::upload_logo(&ctx, &content).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn logo_remove(core: State<'_, Core>) -> Res<Branding> {
    let ctx = core.require(Module::Settings, Action::Manage).await?;
    company::remove_logo(&ctx).await
}
