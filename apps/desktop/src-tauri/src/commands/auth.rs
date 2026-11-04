use nucleo_core::modules::auth::service::{self as auth, Me};
use nucleo_core::modules::company::models::{Branding, OnboardingInput, SetupStatus};
use nucleo_core::modules::company::service as company;
use nucleo_core::modules::users::models::RegisterInput;
use nucleo_core::modules::users::service as users;
use nucleo_core::Core;
use tauri::State;

use super::Res;

/// Sem login: diz se é a primeira abertura (mostra o onboarding).
#[tauri::command(rename_all = "snake_case")]
pub async fn setup_status(core: State<'_, Core>) -> Res<SetupStatus> {
    company::status(&core).await
}

/// Sem login: nome, cores e logo da empresa para a tela de entrada.
#[tauri::command(rename_all = "snake_case")]
pub async fn login_branding(core: State<'_, Core>) -> Res<Option<Branding>> {
    company::public_branding(&core).await
}

/// Sem login: pedido de acesso, que fica aguardando aprovação.
#[tauri::command(rename_all = "snake_case")]
pub async fn register(core: State<'_, Core>, data: RegisterInput) -> Res<()> {
    users::register(&core, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn onboard(core: State<'_, Core>, data: OnboardingInput) -> Res<Me> {
    company::onboard(&core, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn login(core: State<'_, Core>, email: String, password: String) -> Res<Me> {
    auth::login(&core, &email, password).await
}

#[tauri::command(rename_all = "snake_case")]
pub fn logout(core: State<'_, Core>) {
    auth::logout(&core);
}

#[tauri::command(rename_all = "snake_case")]
pub async fn me(core: State<'_, Core>) -> Res<Me> {
    let ctx = core.current().await?;
    auth::me(&ctx).await
}

/// Planos (preço e limites). Hoje tudo está liberado; usado na página "Planos".
#[tauri::command(rename_all = "snake_case")]
pub async fn plans_catalog(core: State<'_, Core>) -> Res<Vec<nucleo_core::plans::CatalogItem>> {
    core.current().await?;
    Ok(nucleo_core::plans::catalog())
}
