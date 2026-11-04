use serde::{Deserialize, Serialize};

use crate::modules::double_option;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub document: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub notes: Option<String>,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CustomerInput {
    pub name: String,
    pub document: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub notes: Option<String>,
}

/// Edição parcial (uma célula da planilha = um campo); `null` limpa o campo.
#[derive(Debug, Default, Deserialize)]
pub struct CustomerPatch {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub document: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub phone: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub email: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub city: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub state: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
    pub active: Option<bool>,
}
