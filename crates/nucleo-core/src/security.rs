//! Senhas com argon2 (rodado fora do executor async: é CPU-pesado de propósito).

use std::sync::OnceLock;

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand_core::OsRng;

use crate::error::{CoreError, CoreResult};

fn hash_sync(password: &str) -> CoreResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| CoreError::Internal(format!("argon2: {e}")))
}

/// Hash fixo para gastar o mesmo tempo quando o e-mail não existe (evita enumeração).
fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| hash_sync("nucleo-dummy-password").unwrap_or_default())
}

fn verify_sync(password: &str, hash: Option<&str>) -> bool {
    let target = hash.unwrap_or_else(|| dummy_hash());
    let ok = PasswordHash::new(target)
        .map(|h| {
            Argon2::default()
                .verify_password(password.as_bytes(), &h)
                .is_ok()
        })
        .unwrap_or(false);
    ok && hash.is_some()
}

pub async fn hash_password(password: String) -> CoreResult<String> {
    tokio::task::spawn_blocking(move || hash_sync(&password))
        .await
        .map_err(|e| CoreError::Internal(e.to_string()))?
}

pub async fn verify_password(password: String, hash: Option<String>) -> bool {
    tokio::task::spawn_blocking(move || verify_sync(&password, hash.as_deref()))
        .await
        .unwrap_or(false)
}
