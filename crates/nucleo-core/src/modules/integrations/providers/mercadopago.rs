//! Mercado Pago (PIX), API de pagamentos `/v1/payments`. Teste e produção usam o mesmo
//! endereço; o que muda é o Access Token (credenciais de teste × produção).
//! Docs: <https://www.mercadopago.com.br/developers/pt/docs/checkout-api-v2/payment-integration/pix>

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};

use super::pix_common::{self as pix, CreateReq};
use crate::modules::integrations::catalog::{Field, Kind, ProviderInfo};
use crate::modules::integrations::models::Job;
use crate::modules::integrations::provider::{Call, Outcome, Provider, ProviderError};

pub const ID: &str = "mercadopago";
const BASE: &str = "https://api.mercadopago.com";

pub struct MercadoPago;

fn token(call: &Call<'_>) -> Result<String, ProviderError> {
    call.secret("access_token")
        .filter(|t| !t.is_empty())
        .ok_or_else(|| ProviderError::Auth("sem access token".into()))
}

/// Corpo do pagamento PIX.
pub fn payment_body(req: &CreateReq, fallback_email: &str, expires_at: &str) -> Value {
    let payer = req.payer.as_ref();
    let email = payer
        .and_then(|p| p.email.as_deref())
        .filter(|e| e.contains('@'))
        .unwrap_or(fallback_email);
    let mut payer_json = json!({ "email": email });
    if let Some(name) = payer.map(|p| p.name.trim()).filter(|n| !n.is_empty()) {
        payer_json["first_name"] = json!(name);
    }
    if let Some(doc) = req.payer_document() {
        let kind = if doc.len() == 11 { "CPF" } else { "CNPJ" };
        payer_json["identification"] = json!({ "type": kind, "number": doc });
    }
    let amount: Value = serde_json::from_str(&pix::amount(req.amount_cents)).unwrap_or(json!(0));
    json!({
        "transaction_amount": amount,
        "description": req.description,
        "payment_method_id": "pix",
        "external_reference": req.txid,
        "date_of_expiration": expires_at,
        "payer": payer_json,
    })
}

/// Resposta da criação → resultado do `pix.create`.
pub fn create_result(body: &Value, expires_at: &str) -> Result<Value, ProviderError> {
    let id = match &body["id"] {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => return Err(ProviderError::Rejected("pagamento sem id".into())),
    };
    let code = body["point_of_interaction"]["transaction_data"]["qr_code"]
        .as_str()
        .ok_or_else(|| {
            ProviderError::Rejected("o Mercado Pago não devolveu o código PIX".into())
        })?;
    let expires = body["date_of_expiration"]
        .as_str()
        .and_then(pix::parse)
        .map(pix::iso)
        .unwrap_or_else(|| expires_at.to_owned());
    Ok(json!({ "provider_ref": id, "copy_paste": code, "expires_at": expires }))
}

/// Situação do pagamento → resultado da consulta.
pub fn poll_outcome(body: &Value, job: &Job) -> Outcome {
    match body["status"].as_str().unwrap_or_default() {
        "approved" | "authorized" => pix::paid(body["date_approved"].as_str()),
        "cancelled" | "rejected" | "refunded" | "charged_back" => {
            Outcome::Done(json!({ "status": "expired" }))
        }
        _ => pix::not_paid_yet(job),
    }
}

impl MercadoPago {
    async fn cancel(&self, call: &Call<'_>, id: &str) -> Result<(), ProviderError> {
        let req = call
            .http
            .client()
            .put(format!("{BASE}/v1/payments/{id}"))
            .bearer_auth(token(call)?)
            .json(&json!({ "status": "cancelled" }));
        match call.http.send(req).await {
            Ok(_) | Err(ProviderError::Rejected(_)) => Ok(()), // já cancelado/expirado
            Err(e) => Err(e),
        }
    }
}

#[async_trait]
impl Provider for MercadoPago {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            fields: vec![
                Field::secret("access_token", "Access Token", true).help(
                    "Mercado Pago Developers → Suas integrações → Credenciais. Use o de teste no ambiente Teste.",
                ),
                Field::text("email_pagador", "E-mail do pagador padrão", true).help(
                    "O Mercado Pago exige um e-mail de quem paga. Usado quando o cliente não tem e-mail cadastrado (não pode ser o e-mail da sua conta).",
                ),
            ],
            capabilities: vec!["pix"],
            ..ProviderInfo::new(
                ID,
                "Mercado Pago",
                Kind::Pix,
                "PIX na venda pela sua conta Mercado Pago: QR Code na tela e baixa automática quando o cliente paga.",
            )
        }
    }

    async fn check(&self, call: &Call<'_>) -> Result<String, ProviderError> {
        let req = call
            .http
            .client()
            .get(format!("{BASE}/users/me"))
            .bearer_auth(token(call)?);
        let body = pix::json_body(call.http.send(req).await?).await?;
        let who = body["nickname"]
            .as_str()
            .or(body["email"].as_str())
            .unwrap_or("conta conectada");
        Ok(format!("Conta Mercado Pago: {who}"))
    }

    async fn run(&self, call: &Call<'_>, job: &Job) -> Outcome {
        let token = match token(call) {
            Ok(t) => t,
            Err(e) => return Outcome::Error(e),
        };
        match job.kind.as_str() {
            "pix.create" => {
                let req = match CreateReq::from_job(job) {
                    Ok(r) => r,
                    Err(e) => return Outcome::Error(e),
                };
                let expires = (Utc::now() + chrono::Duration::seconds(req.expires_secs))
                    .format("%Y-%m-%dT%H:%M:%S%.3f%:z")
                    .to_string();
                let fallback = call.config_str("email_pagador").unwrap_or_default();
                let http = call
                    .http
                    .client()
                    .post(format!("{BASE}/v1/payments"))
                    .bearer_auth(&token)
                    // mesma chave nas novas tentativas: o Mercado Pago não duplica o pagamento
                    .header("X-Idempotency-Key", &req.txid)
                    .json(&payment_body(&req, fallback, &expires));
                let result = async {
                    let body = pix::json_body(call.http.send(http).await?).await?;
                    create_result(&body, &req.expires_at())
                };
                match result.await {
                    Ok(v) => Outcome::Done(v),
                    Err(e) => Outcome::Error(e),
                }
            }
            "pix.poll" => {
                let id = match pix::provider_ref(job) {
                    Ok(id) => id,
                    Err(e) => return Outcome::Error(e),
                };
                let req = call
                    .http
                    .client()
                    .get(format!("{BASE}/v1/payments/{id}"))
                    .bearer_auth(&token);
                let body = match call.http.send(req).await {
                    Ok(r) => pix::json_body(r).await,
                    Err(e) => Err(e),
                };
                match body {
                    Ok(b) => {
                        let out = poll_outcome(&b, job);
                        if out == Outcome::Done(json!({ "status": "expired" }))
                            && b["status"] == "pending"
                        {
                            // venceu do nosso lado: fecha no Mercado Pago para não ser pago depois
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
    use crate::modules::pix::models::Payer;

    fn req(payer: Option<Payer>) -> CreateReq {
        CreateReq {
            txid: "NUC123".into(),
            amount_cents: 1990,
            description: "Venda nº 7".into(),
            expires_secs: 3600,
            payer,
        }
    }

    #[test]
    fn body_uses_customer_or_fallback_email_and_document() {
        let b = payment_body(
            &req(None),
            "caixa@loja.com",
            "2026-11-04T15:00:00.000+00:00",
        );
        assert_eq!(b["payer"]["email"], "caixa@loja.com");
        assert_eq!(b["transaction_amount"].to_string(), "19.9");
        assert_eq!(b["payment_method_id"], "pix");
        assert_eq!(b["external_reference"], "NUC123");

        let p = Payer {
            name: "Ana".into(),
            document: Some("123.456.789-09".into()),
            email: Some("ana@x.com".into()),
        };
        let b = payment_body(&req(Some(p)), "caixa@loja.com", "x");
        assert_eq!(b["payer"]["email"], "ana@x.com");
        assert_eq!(b["payer"]["identification"]["type"], "CPF");
        assert_eq!(b["payer"]["identification"]["number"], "12345678909");
    }

    #[test]
    fn parses_creation_and_status() {
        let body = json!({
            "id": 123456789,
            "status": "pending",
            "date_of_expiration": "2026-11-04T15:00:00.000-03:00",
            "point_of_interaction": { "transaction_data": { "qr_code": "00020126…6304ABCD" } }
        });
        let r = create_result(&body, "fallback").unwrap();
        assert_eq!(r["provider_ref"], "123456789");
        assert_eq!(r["expires_at"], "2026-11-04T18:00:00.000Z");
        assert!(create_result(&json!({ "id": 1 }), "x").is_err());

        let job = Job {
            id: "j".into(),
            integration_id: "i".into(),
            kind: "pix.poll".into(),
            payload: json!({ "expires_at": "2999-01-01T00:00:00.000Z" }),
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
        };
        assert!(matches!(
            poll_outcome(&json!({ "status": "pending" }), &job),
            Outcome::Wait { .. }
        ));
        match poll_outcome(
            &json!({ "status": "approved", "date_approved": "2026-11-04T12:00:00.000-03:00" }),
            &job,
        ) {
            Outcome::Done(v) => {
                assert_eq!(v["status"], "paid");
                assert_eq!(v["paid_at"], "2026-11-04T15:00:00.000Z");
            }
            other => panic!("{other:?}"),
        }
    }
}
