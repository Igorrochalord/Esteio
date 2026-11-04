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
