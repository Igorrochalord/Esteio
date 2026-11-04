//! Partes comuns aos provedores de PIX (contrato em `modules::pix`).

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use crate::modules::integrations::models::Job;
use crate::modules::integrations::provider::{Outcome, ProviderError};
use crate::modules::pix::models::Payer;

/// Intervalo entre consultas enquanto o PIX não é pago.
pub const POLL_SECS: i64 = 5;
/// Depois do vencimento, ainda consulta por este tempo (pagamento no último segundo).
pub const GRACE_SECS: i64 = 120;

pub fn iso(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

pub fn parse(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// Centavos → "12.34" (formato das APIs).
pub fn amount(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// Dados do `pix.create`.
pub struct CreateReq {
    pub txid: String,
    pub amount_cents: i64,
    pub description: String,
    pub expires_secs: i64,
    pub payer: Option<Payer>,
}

impl CreateReq {
    pub fn from_job(job: &Job) -> Result<Self, ProviderError> {
        let p = &job.payload;
        let amount_cents = p["amount_cents"].as_i64().unwrap_or(0);
        if amount_cents <= 0 {
            return Err(ProviderError::Rejected("valor inválido".into()));
        }
        Ok(Self {
            txid: p["txid"].as_str().unwrap_or_default().to_owned(),
            amount_cents,
            description: p["description"].as_str().unwrap_or("Venda").to_owned(),
            expires_secs: p["expires_secs"]
                .as_i64()
                .unwrap_or(3600)
                .clamp(1800, 86_400),
            payer: serde_json::from_value(p["payer"].clone()).ok(),
        })
    }

    pub fn expires_at(&self) -> String {
        iso(Utc::now() + chrono::Duration::seconds(self.expires_secs))
    }

    pub fn payer_document(&self) -> Option<String> {
        self.payer
            .as_ref()
            .and_then(|p| p.document.as_deref())
            .map(|d| d.chars().filter(char::is_ascii_digit).collect::<String>())
            .filter(|d| d.len() == 11 || d.len() == 14)
    }
}

/// Consulta ainda não paga: espera de novo, ou dá como expirada depois do prazo + folga.
pub fn not_paid_yet(job: &Job) -> Outcome {
    let expired = job.payload["expires_at"]
        .as_str()
        .and_then(parse)
        .is_some_and(|e| Utc::now() > e + chrono::Duration::seconds(GRACE_SECS));
    if expired {
        Outcome::Done(json!({ "status": "expired" }))
    } else {
        Outcome::Wait {
            secs: POLL_SECS,
            note: Some("Aguardando pagamento".into()),
            payload: None,
        }
    }
}

pub fn paid(paid_at: Option<&str>) -> Outcome {
    let at = paid_at
        .and_then(parse)
        .map(iso)
        .unwrap_or_else(|| iso(Utc::now()));
    Outcome::Done(json!({ "status": "paid", "paid_at": at }))
}

pub fn provider_ref(job: &Job) -> Result<String, ProviderError> {
    job.payload["provider_ref"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ProviderError::Rejected("cobrança sem identificador do serviço".into()))
}

/// Lê um JSON de resposta (erros de formato viram recusa, não repetem).
pub async fn json_body(resp: reqwest::Response) -> Result<Value, ProviderError> {
    resp.json::<Value>()
        .await
        .map_err(|e| ProviderError::Rejected(format!("resposta inesperada: {e}")))
}

pub fn unknown(kind: &str) -> Outcome {
    Outcome::Error(ProviderError::Rejected(format!(
        "este serviço não sabe fazer \"{kind}\""
    )))
}
