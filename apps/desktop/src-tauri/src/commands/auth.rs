use nucleo_core::modules::auth::service::{self as auth, Me};
use nucleo_core::modules::company::models::{OnboardingInput, SetupStatus};
use nucleo_core::modules::company::service as company;
use nucleo_core::Core;
use tauri::State;

use super::Res;

/// Sem login: diz se é a primeira abertura (mostra o onboarding).
#[tauri::command(rename_all = "snake_case")]
pub async fn setup_status(core: State<'_, Core>) -> Res<SetupStatus> {
    company::status(&core).await
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
