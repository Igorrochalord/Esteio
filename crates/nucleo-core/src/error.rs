//! Erros de domínio. Services devolvem `CoreError`; a camada Tauri serializa como
//! `{ code, message, fields }` e o frontend mostra `message` ao usuário.

use std::collections::BTreeMap;

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    BusinessRule(String),
    /// campo → mensagem (ex.: "owner.email" → "E-mail inválido")
    #[error("Confira os campos destacados.")]
    Validation(BTreeMap<String, String>),
    /// detalhe técnico só no log; o usuário vê uma mensagem genérica
    #[error("Ocorreu um erro inesperado. Tente novamente.")]
    Internal(String),
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Forbidden(_) => "forbidden",
            Self::Unauthorized(_) => "unauthorized",
            Self::BusinessRule(_) => "business_rule",
            Self::Validation(_) => "validation",
            Self::Internal(_) => "internal",
        }
    }

    pub fn field(name: &str, message: &str) -> Self {
        Self::Validation(BTreeMap::from([(name.to_owned(), message.to_owned())]))
    }
}

impl Serialize for CoreError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("CoreError", 3)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        let empty = BTreeMap::new();
        let fields = match self {
            Self::Validation(f) => f,
            _ => &empty,
        };
        st.serialize_field("fields", fields)?;
        st.end()
    }
}

impl From<sqlx::Error> for CoreError {
    fn from(e: sqlx::Error) -> Self {
        log::error!("erro de banco: {e:?}");
        Self::Internal(e.to_string())
    }
}

impl From<sqlx::migrate::MigrateError> for CoreError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        log::error!("erro de migração: {e:?}");
        Self::Internal(e.to_string())
    }
}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        log::error!("erro de arquivo: {e:?}");
        Self::Internal(e.to_string())
    }
}

/// Acumula erros de validação por campo (equivalente aos validadores do Pydantic).
#[derive(Default)]
pub struct Validator(BTreeMap<String, String>);

impl Validator {
    pub fn check(&mut self, ok: bool, field: &str, message: &str) {
        if !ok && !self.0.contains_key(field) {
            self.0.insert(field.to_owned(), message.to_owned());
        }
    }

    pub fn add(&mut self, field: &str, message: &str) {
        self.check(false, field, message);
    }

    pub fn finish(self) -> CoreResult<()> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(CoreError::Validation(self.0))
        }
    }
}
