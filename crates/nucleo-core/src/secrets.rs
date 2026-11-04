//! Cofre local para credenciais de integrações (tokens, senhas de API).
//!
//! As credenciais vão cifradas para o banco (ChaCha20-Poly1305). A chave fica num arquivo
//! **fora** do banco (`<dados>/integracoes.key`): uma cópia de segurança do banco levada para
//! outro computador não carrega tokens utilizáveis; lá, basta informar as credenciais de novo.

use std::path::Path;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand_core::{OsRng, RngCore};

use crate::error::{CoreError, CoreResult};

const KEY_FILE: &str = "integracoes.key";
const NONCE_LEN: usize = 12;

#[derive(Clone)]
pub struct Vault {
    key: [u8; 32],
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Vault(..)")
    }
}

impl Vault {
    /// Lê a chave de `<dir>/integracoes.key` ou cria uma nova (permissão 0600 no Unix).
    pub fn open(dir: &Path) -> CoreResult<Self> {
        let path = dir.join(KEY_FILE);
        if let Ok(bytes) = std::fs::read(&path) {
            let key: [u8; 32] = bytes.try_into().map_err(|_| {
                CoreError::Internal(format!("{} corrompido (tamanho inválido)", path.display()))
            })?;
            return Ok(Self { key });
        }
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        write_private(&path, &key)?;
        Ok(Self { key })
    }

    pub fn seal(&self, plain: &[u8]) -> CoreResult<Vec<u8>> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&self.key));
        let mut nonce = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce);
        let mut out = nonce.to_vec();
        out.extend(
            cipher
                .encrypt(Nonce::from_slice(&nonce), plain)
                .map_err(|_| CoreError::Internal("falha ao cifrar credenciais".into()))?,
        );
        Ok(out)
    }

    /// `None` quando o dado foi cifrado com outra chave (ex.: banco restaurado de outro PC).
    pub fn open_sealed(&self, sealed: &[u8]) -> Option<Vec<u8>> {
        if sealed.len() <= NONCE_LEN {
            return None;
        }
        let (nonce, data) = sealed.split_at(NONCE_LEN);
        ChaCha20Poly1305::new(Key::from_slice(&self.key))
            .decrypt(Nonce::from_slice(nonce), data)
            .ok()
    }
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> CoreResult<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(bytes)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> CoreResult<()> {
    // Windows: a pasta %APPDATA% já é privada do usuário
    std::fs::write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_roundtrip_and_foreign_key_fails() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let va = Vault::open(a.path()).unwrap();
        let sealed = va.seal(b"token-secreto").unwrap();
        assert!(!sealed.windows(5).any(|w| w == b"token"));
        assert_eq!(va.open_sealed(&sealed).unwrap(), b"token-secreto");
        // mesma pasta = mesma chave
        assert_eq!(
            Vault::open(a.path()).unwrap().open_sealed(&sealed).unwrap(),
            b"token-secreto"
        );
        // outro computador
        assert!(Vault::open(b.path())
            .unwrap()
            .open_sealed(&sealed)
            .is_none());
    }
}
