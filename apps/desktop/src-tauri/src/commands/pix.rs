//! PIX na venda. Quem vende (Vendas: escrever) gera e acompanha a cobrança; configurar o
//! serviço é da tela de Integrações.

use nucleo_core::modules::pix::models::{PixCharge, PixProviderOption};
use nucleo_core::modules::pix::service as pix;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tauri::State;

use super::Res;

#[tauri::command(rename_all = "snake_case")]
pub async fn pix_providers(core: State<'_, Core>) -> Res<Vec<PixProviderOption>> {
    let ctx = core.require(Module::Sales, Action::Read).await?;
    pix::providers(&core, &ctx).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn pix_for_sale(core: State<'_, Core>, sale_id: String) -> Res<Option<PixCharge>> {
    let ctx = core.require(Module::Sales, Action::Read).await?;
    pix::for_sale(&core, &ctx, &sale_id).await
}

/// "Já pagou?": roda a geração/consulta pendente agora.
#[tauri::command(rename_all = "snake_case")]
pub async fn pix_check(core: State<'_, Core>, id: String) -> Res<PixCharge> {
    let ctx = core.require(Module::Sales, Action::Read).await?;
    pix::check(&core, &ctx, &id).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn pix_cancel(core: State<'_, Core>, id: String) -> Res<PixCharge> {
    let ctx = core.require(Module::Sales, Action::Write).await?;
    pix::cancel(&core, &ctx, &id).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn pix_new(core: State<'_, Core>, sale_id: String, provider: String) -> Res<PixCharge> {
    let ctx = core.require(Module::Sales, Action::Write).await?;
    pix::new_for_sale(&core, &ctx, &sale_id, &provider).await
}
