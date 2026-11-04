//! Asaas (PIX): cobrança `billingType = PIX` + QR dinâmico. Sandbox e produção têm endereços e
//! chaves diferentes. A conta precisa de uma chave Pix cadastrada.
//! Docs: <https://docs.asaas.com/docs/cobrancas-via-pix>

use async_trait::async_trait;
use chrono::{FixedOffset, Utc};
use serde_json::{json, Value};

use super::pix_common::{self as pix, CreateReq};
use crate::modules::integrations::catalog::{Field, Kind, ProviderInfo};
use crate::modules::integrations::models::Job;
use crate::modules::integrations::provider::{Call, Outcome, Provider, ProviderError};

pub const ID: &str = "asaas";

pub struct Asaas;

fn base(call: &Call<'_>) -> &'static str {
    if call.production {
        "https://api.asaas.com/v3"
    } else {
        "https://api-sandbox.asaas.com/v3"
    }
}

fn key(call: &Call<'_>) -> Result<String, ProviderError> {
    call.secret("api_key")
        .filter(|t| !t.is_empty())
        .ok_or_else(|| ProviderError::Auth("sem chave de API".into()))
}

fn digits(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}

/// Data de hoje em Brasília (vencimento da cobrança).
fn today_brt() -> String {
    let brt = FixedOffset::west_opt(3 * 3600).expect("fuso válido");
    Utc::now()
        .with_timezone(&brt)
        .format("%Y-%m-%d")
        .to_string()
}

/// Situação da cobrança → resultado da consulta.
pub fn poll_outcome(body: &Value, job: &Job) -> Outcome {
    if body["deleted"].as_bool() == Some(true) {
        return Outcome::Done(json!({ "status": "canceled" }));
    }
    match body["status"].as_str().unwrap_or_default() {
        "RECEIVED" | "CONFIRMED" | "RECEIVED_IN_CASH" => pix::paid(None),
        "REFUNDED" | "REFUND_REQUESTED" | "CHARGEBACK_REQUESTED" => {
            Outcome::Done(json!({ "status": "canceled" }))
        }
        _ => pix::not_paid_yet(job),
    }
}

impl Asaas {
    fn req(
        &self,
        call: &Call<'_>,
        method: reqwest::Method,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, ProviderError> {
        Ok(call
            .http
            .client()
            .request(method, format!("{}{path}", base(call)))
            .header("access_token", key(call)?))
    }

    async fn get(&self, call: &Call<'_>, path: &str) -> Result<Value, ProviderError> {
        let r = self.req(call, reqwest::Method::GET, path)?;
        pix::json_body(call.http.send(r).await?).await
    }

    /// Cliente do Asaas pelo CPF/CNPJ (cria se não existir).
    async fn customer(&self, call: &Call<'_>, req: &CreateReq) -> Result<String, ProviderError> {
        let (doc, name, email) = match req.payer_document() {
            Some(doc) => {
                let p = req.payer.as_ref();
                (
                    doc,
                    p.map(|p| p.name.clone())
                        .unwrap_or_else(|| "Cliente".into()),
                    p.and_then(|p| p.email.clone()),
                )
            }
            None => {
                let doc = call
                    .config_str("cpf_cnpj_padrao")
                    .map(digits)
                    .unwrap_or_default();
                if doc.len() != 11 && doc.len() != 14 {
                    return Err(ProviderError::Rejected(
                        "o Asaas exige CPF/CNPJ de quem paga: cadastre o documento do cliente ou informe o CPF/CNPJ padrão na integração".into(),
                    ));
                }
                (doc, "Consumidor".into(), None)
            }
        };
        let found = self
            .get(call, &format!("/customers?cpfCnpj={doc}&limit=1"))
            .await?;
        if let Some(id) = found["data"][0]["id"].as_str() {
            return Ok(id.to_owned());
        }
        let mut body = json!({ "name": name, "cpfCnpj": doc, "notificationDisabled": true });
        if let Some(e) = email.filter(|e| e.contains('@')) {
            body["email"] = json!(e);
        }
        let r = self
            .req(call, reqwest::Method::POST, "/customers")?
            .json(&body);
        let created = pix::json_body(call.http.send(r).await?).await?;
        created["id"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| ProviderError::Rejected("cliente sem id".into()))
    }

    async fn create(&self, call: &Call<'_>, job: &Job) -> Result<Value, ProviderError> {
        let req = CreateReq::from_job(job)?;
        // o Asaas não tem chave de idempotência: reaproveita a cobrança de uma tentativa anterior
        let existing = self
            .get(
                call,
                &format!("/payments?externalReference={}&limit=1", req.txid),
            )
            .await?;
        let id = match existing["data"][0]["id"].as_str() {
            Some(id) => id.to_owned(),
            None => {
                let customer = self.customer(call, &req).await?;
                let amount: Value =
                    serde_json::from_str(&pix::amount(req.amount_cents)).unwrap_or(json!(0));
                let body = json!({
                    "customer": customer,
                    "billingType": "PIX",
                    "value": amount,
                    "dueDate": today_brt(),
                    "description": req.description,
                    "externalReference": req.txid,
                });
                let r = self
                    .req(call, reqwest::Method::POST, "/payments")?
                    .json(&body);
                let created = pix::json_body(call.http.send(r).await?).await?;
                created["id"]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| ProviderError::Rejected("cobrança sem id".into()))?
            }
        };
        let qr = self.get(call, &format!("/payments/{id}/pixQrCode")).await?;
        let code = qr["payload"].as_str().ok_or_else(|| {
            ProviderError::Rejected(
                "o Asaas não devolveu o código PIX (confira se a conta tem chave Pix cadastrada)"
                    .into(),
            )
        })?;
        Ok(json!({ "provider_ref": id, "copy_paste": code, "expires_at": req.expires_at() }))
    }

    async fn cancel(&self, call: &Call<'_>, id: &str) -> Result<(), ProviderError> {
        let r = self.req(call, reqwest::Method::DELETE, &format!("/payments/{id}"))?;
        match call.http.send(r).await {
            Ok(_) | Err(ProviderError::Rejected(_)) => Ok(()), // já paga/removida
            Err(e) => Err(e),
        }
    }
}

#[async_trait]
impl Provider for Asaas {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            fields: vec![
                Field::secret("api_key", "Chave de API", true).help(
                    "No Asaas: Integrações → Chave de API. O sandbox tem conta e chave próprias.",
                ),
                Field::text("cpf_cnpj_padrao", "CPF/CNPJ padrão de quem paga", false).help(
                    "O Asaas exige CPF/CNPJ do pagador. Usado quando a venda não tem cliente com documento.",
                ),
            ],
            capabilities: vec!["pix"],
            ..ProviderInfo::new(
                ID,
                "Asaas",
                Kind::Pix,
                "PIX na venda pela sua conta Asaas: QR Code na tela e baixa automática quando o cliente paga.",
            )
        }
    }

    async fn check(&self, call: &Call<'_>) -> Result<String, ProviderError> {
        self.get(call, "/finance/balance").await?;
        Ok(if call.production {
            "Conta Asaas conectada.".into()
        } else {
            "Conta Asaas (sandbox) conectada.".into()
        })
    }

    async fn run(&self, call: &Call<'_>, job: &Job) -> Outcome {
        match job.kind.as_str() {
            "pix.create" => match self.create(call, job).await {
                Ok(v) => Outcome::Done(v),
                Err(e) => Outcome::Error(e),
            },
            "pix.poll" => {
                let id = match pix::provider_ref(job) {
                    Ok(id) => id,
                    Err(e) => return Outcome::Error(e),
                };
                match self.get(call, &format!("/payments/{id}")).await {
                    Ok(b) => {
                        let out = poll_outcome(&b, job);
                        if out == Outcome::Done(json!({ "status": "expired" })) {
                            let _ = self.cancel(call, &id).await;
                        }
                        out
                    }
                    Err(e) => Outcome::Error(e),
                }
            }
            "pix.cancel" => match pix::provider_ref(job) {
                Ok(id) => match self.cancel(call, &id).await {
                    Ok(()) => Outcome::Done(json!({ "status": "canceled" })),
                    Err(e) => Outcome::Error(e),
                },
                Err(_) => Outcome::Done(json!({ "status": "canceled" })),
            },
            other => pix::unknown(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job() -> Job {
        Job {
            id: "j".into(),
            integration_id: "i".into(),
            kind: "pix.poll".into(),
            payload: json!({ "expires_at": "2000-01-01T00:00:00.000Z" }),
            status: "running".into(),
            attempts: 0,
            next_run_at: String::new(),
            last_error: None,
            result: None,
            ref_entity: None,
            ref_id: None,
            finished_at: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn status_mapping() {
        let j = job();
        match poll_outcome(&json!({ "status": "RECEIVED" }), &j) {
            Outcome::Done(v) => assert_eq!(v["status"], "paid"),
            o => panic!("{o:?}"),
        }
        assert_eq!(
            poll_outcome(&json!({ "status": "PENDING", "deleted": true }), &j),
            Outcome::Done(json!({ "status": "canceled" }))
        );
        // pendente depois do prazo + folga: expira
        assert_eq!(
            poll_outcome(&json!({ "status": "PENDING" }), &j),
            Outcome::Done(json!({ "status": "expired" }))
        );
        assert_eq!(today_brt().len(), 10);
    }
}
