use nucleo_core::modules::users::models::{User, UserCreate, UserUpdate};
use nucleo_core::modules::users::service as users;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tauri::State;

use super::Res;

#[tauri::command(rename_all = "snake_case")]
pub async fn users_list(core: State<'_, Core>) -> Res<Vec<User>> {
    let ctx = core.require(Module::Users, Action::Read).await?;
    users::list(&ctx).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn users_create(core: State<'_, Core>, data: UserCreate) -> Res<User> {
    let ctx = core.require(Module::Users, Action::Manage).await?;
    users::create(&ctx, data).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn users_update(core: State<'_, Core>, user_id: String, data: UserUpdate) -> Res<User> {
    let ctx = core.require(Module::Users, Action::Manage).await?;
    users::update(&ctx, &user_id, data).await
}
