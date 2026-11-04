//! Cliente HTTP compartilhado pelos provedores (rustls; sem OpenSSL no instalador).

use std::time::Duration;

use super::provider::ProviderError;

#[derive(Clone)]
pub struct Http {
    client: reqwest::Client,
}

impl std::fmt::Debug for Http {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Http")
    }
}

impl Http {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!("NucleoERP/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Envia e separa sem internet / credencial recusada / erro da API.
    pub async fn send(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, ProviderError> {
        let resp = req.send().await.map_err(classify)?;
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        let body = resp.text().await.unwrap_or_default();
        let body: String = body.chars().take(500).collect();
        Err(match status.as_u16() {
            401 | 403 => ProviderError::Auth(body),
            408 | 429 | 500..=599 => ProviderError::Unavailable(format!("HTTP {status}: {body}")),
            _ => ProviderError::Rejected(format!("HTTP {status}: {body}")),
        })
    }
}

impl Http {
    /// Cliente com certificado do cliente (mTLS), a partir de um .p12/.pfx e sua senha.
    /// Usado pela Efí (PIX) e pela NFS-e Nacional (certificado A1).
    pub fn with_pkcs12(&self, der: &[u8], password: &str) -> Result<Http, ProviderError> {
        let cert = Pkcs12Pem::parse(der, password)?;
        let identity = reqwest::Identity::from_pem(cert.pem.as_bytes())
            .map_err(|e| ProviderError::Rejected(format!("certificado inválido: {e}")))?;
        let client = reqwest::Client::builder()
            .user_agent(concat!("NucleoERP/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .identity(identity)
            .build()
            .map_err(|e| ProviderError::Rejected(format!("certificado inválido: {e}")))?;
        Ok(Http { client })
    }
}

/// Certificado .p12/.pfx convertido para PEM (chave PKCS#8 + cadeia).
pub struct Pkcs12Pem {
    pub pem: String,
    /// titular do certificado (para mostrar na tela)
    pub subject: String,
}

impl Pkcs12Pem {
    pub fn parse(der: &[u8], password: &str) -> Result<Self, ProviderError> {
        use p12_keystore::{KeyStore, Pkcs12ImportPolicy};
        let invalid = || {
            ProviderError::Rejected(
                "não foi possível abrir o certificado: confira o arquivo e a senha".into(),
            )
        };
        let store = KeyStore::from_pkcs12(der, password, Pkcs12ImportPolicy::Relaxed)
            .map_err(|_| invalid())?;
        let (_, chain) = store.private_key_chain().ok_or_else(invalid)?;
        let mut pem = pem_block("PRIVATE KEY", chain.key().as_der());
        for c in chain.certs() {
            pem.push_str(&pem_block("CERTIFICATE", c.as_der()));
        }
        let subject = chain
            .certs()
            .first()
            .map(|c| c.subject().to_owned())
            .unwrap_or_default();
        Ok(Self { pem, subject })
    }
}

fn pem_block(label: &str, der: &[u8]) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(der);
    let mut out = format!("-----BEGIN {label}-----\n");
    for line in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(line).unwrap_or_default());
        out.push('\n');
    }
    out.push_str(&format!("-----END {label}-----\n"));
    out
}

impl Default for Http {
    fn default() -> Self {
        Self::new()
    }
}

pub fn classify(e: reqwest::Error) -> ProviderError {
    if e.is_connect() || e.is_timeout() {
        ProviderError::Offline
    } else if e.is_decode() {
        ProviderError::Rejected(format!("resposta inesperada: {e}"))
    } else {
        ProviderError::Unavailable(e.to_string())
    }
}
