use serde::{Deserialize, Serialize};

use crate::permissions::Role;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    /// autocadastro aguardando aprovação de um administrador
    pub pending: bool,
    pub last_login_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// recusa de autocadastro = exclusão lógica
    #[serde(skip_serializing)]
    pub deleted_at: Option<String>,
}

impl User {
    pub fn role(&self) -> Role {
        Role::parse(&self.role).unwrap_or(Role::User)
    }
}

/// owner só nasce no onboarding
fn default_role() -> String {
    "user".into()
}

#[derive(Debug, Deserialize)]
pub struct UserCreate {
    pub name: String,
    pub email: String,
    /// senha provisória
    pub password: String,
    #[serde(default = "default_role")]
    pub role: String,
}

/// Autocadastro pela tela de login (sem sessão).
#[derive(Debug, Deserialize)]
pub struct RegisterInput {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct UserUpdate {
    pub name: Option<String>,
    pub role: Option<String>,
    pub is_active: Option<bool>,
}
