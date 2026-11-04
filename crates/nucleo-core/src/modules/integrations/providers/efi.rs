//! Efí (PIX), API Pix do Banco Central (`/v2/cob`) com certificado `.p12` (mTLS) e OAuth
//! `client_credentials`. Homologação e produção têm endereços, credenciais e certificados próprios.
//! Docs: <https://dev.efipay.com.br/docs/api-pix/cobrancas-imediatas>

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Map, Value};

use super::pix_common::{self as pix, CreateReq};
use crate::modules::integrations::catalog::{Field, Kind, ProviderInfo};
use crate::modules::integrations::http::{Http, Pkcs12Pem};
use crate::modules::integrations::models::Job;
use crate::modules::integrations::provider::{Call, Outcome, Provider, ProviderError};

pub const ID: &str = "efi";
const REMOVED: &str = "REMOVIDA_PELO_USUARIO_RECEBEDOR";

pub struct Efi;

fn base(call: &Call<'_>) -> &'static str {
    if call.production {
        "https://pix.api.efipay.com.br"
    } else {
        "https://pix-h.api.efipay.com.br"
    }
}

fn need(v: Option<String>, what: &str) -> Result<String, ProviderError> {
    v.filter(|s| !s.is_empty())
        .ok_or_else(|| ProviderError::Auth(format!("falta {what}")))
}

/// Corpo da cobrança imediata (padrão do Banco Central).
pub fn cob_body(req: &CreateReq, key: &str) -> Value {
    let mut body = json!({
        "calendario": { "expiracao": req.expires_secs },
        "valor": { "original": pix::amount(req.amount_cents) },
        "chave": key,
        "solicitacaoPagador": req.description,
    });
    if let (Some(doc), Some(p)) = (req.payer_document(), req.payer.as_ref()) {
        let field = if doc.len() == 11 { "cpf" } else { "cnpj" };
        body["devedor"] = json!({ field: doc, "nome": p.name });
    }
    body
}

/// Situação da cobrança → resultado da consulta.
pub fn poll_outcome(body: &Value, job: &Job) -> Outcome {
    match body["status"].as_str().unwrap_or_default() {
        "CONCLUIDA" => pix::paid(body["pix"][0]["horario"].as_str()),
        s if s.starts_with("REMOVIDA") => Outcome::Done(json!({ "status": "canceled" })),
        _ => pix::not_paid_yet(job),
    }
}

impl Efi {
    /// Cliente HTTP com o certificado da conta.
    fn http(&self, call: &Call<'_>) -> Result<Http, ProviderError> {
        let cert = call
            .file("certificado")
            .ok_or_else(|| ProviderError::Auth("falta o certificado".into()))?;
        let password = call.secret("senha_certificado").unwrap_or_default();
        call.http.with_pkcs12(&cert, &password)
    }

    /// Token de acesso, reaproveitado até perto de vencer (guardado cifrado).
    async fn token(&self, call: &Call<'_>, http: &Http) -> Result<String, ProviderError> {
        let env = if call.production { "p" } else { "h" };
        let cached = call.secret("token").filter(|_| {
            call.secret("token_ambiente").as_deref() == Some(env)
                && call
                    .secret("token_expira")
                    .as_deref()
                    .and_then(pix::parse)
                    .is_some_and(|e| e > Utc::now() + chrono::Duration::seconds(60))
        });
        if let Some(t) = cached {
            return Ok(t);
        }
        let id = need(
            call.config_str("client_id").map(str::to_owned),
            "o Client ID",
        )?;
        let secret = need(call.secret("client_secret"), "o Client Secret")?;
        let req = http
            .client()
            .post(format!("{}/oauth/token", base(call)))
            .basic_auth(id, Some(secret))
            .json(&json!({ "grant_type": "client_credentials" }));
        let body = pix::json_body(http.send(req).await?).await?;
        let token = body["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("a Efí não devolveu o token".into()))?
            .to_owned();
        let secs = body["expires_in"].as_i64().unwrap_or(3600);
        let mut m = Map::new();
        m.insert("token".into(), json!(token));
        m.insert(
            "token_expira".into(),
            json!(pix::iso(Utc::now() + chrono::Duration::seconds(secs))),
        );
        m.insert("token_ambiente".into(), json!(env));
        call.set_secrets(m);
        Ok(token)
    }

    async fn send(
        &self,
        call: &Call<'_>,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, ProviderError> {
        let http = self.http(call)?;
        let token = self.token(call, &http).await?;
        let mut req = http
            .client()
            .request(method, format!("{}{path}", base(call)))
            .bearer_auth(token);
        if let Some(b) = body {
            req = req.json(b);
        }
        pix::json_body(http.send(req).await?).await
    }

    async fn create(&self, call: &Call<'_>, job: &Job) -> Result<Value, ProviderError> {
        let req = CreateReq::from_job(job)?;
        let key = need(
            call.config_str("chave_pix").map(str::to_owned),
            "a chave Pix",
        )
        .map_err(|_| ProviderError::Rejected("informe a chave Pix da conta Efí".into()))?;
        let path = format!("/v2/cob/{}", req.txid);
        let body = match self
            .send(
                call,
                reqwest::Method::PUT,
                &path,
                Some(&cob_body(&req, &key)),
            )
            .await
        {
            Ok(b) => b,
            // nova tentativa de uma cobrança já criada: só consulta
            Err(ProviderError::Rejected(m)) if m.contains("txid_duplicado") => {
                self.send(call, reqwest::Method::GET, &path, None).await?
            }
            Err(e) => return Err(e),
        };
        let code = match body["pixCopiaECola"].as_str() {
            Some(c) => c.to_owned(),
            None => {
                let loc = body["loc"]["id"]
                    .as_i64()
                    .ok_or_else(|| ProviderError::Rejected("cobrança sem location".into()))?;
                let qr = self
                    .send(
                        call,
                        reqwest::Method::GET,
                        &format!("/v2/loc/{loc}/qrcode"),
                        None,
                    )
                    .await?;
                qr["qrcode"]
                    .as_str()
                    .ok_or_else(|| {
                        ProviderError::Rejected("a Efí não devolveu o código PIX".into())
                    })?
                    .to_owned()
            }
        };
        Ok(json!({ "provider_ref": req.txid, "copy_paste": code, "expires_at": req.expires_at() }))
    }

    async fn cancel(&self, call: &Call<'_>, txid: &str) -> Result<(), ProviderError> {
        let body = json!({ "status": REMOVED });
        match self
            .send(
                call,
                reqwest::Method::PATCH,
                &format!("/v2/cob/{txid}"),
                Some(&body),
            )
            .await
        {
            Ok(_) | Err(ProviderError::Rejected(_)) => Ok(()), // já paga/removida
            Err(e) => Err(e),
        }
    }
}

#[async_trait]
impl Provider for Efi {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            fields: vec![
                Field::text("client_id", "Client ID", true)
                    .help("Na conta Efí: API → Aplicações. Homologação e produção têm credenciais próprias."),
                Field::secret("client_secret", "Client Secret", true),
                Field::file("certificado", "Certificado (.p12)", true)
                    .help("API → Meus certificados. Gere o do mesmo ambiente escolhido aqui."),
                Field::secret("senha_certificado", "Senha do certificado", false)
                    .help("Os certificados da Efí normalmente não têm senha."),
                Field::text("chave_pix", "Chave Pix", true)
                    .help("A chave Pix da conta Efí que vai receber (e-mail, telefone, CPF/CNPJ ou aleatória)."),
            ],
            capabilities: vec!["pix"],
            ..ProviderInfo::new(
                ID,
                "Efí",
                Kind::Pix,
                "PIX na venda pela sua conta Efí (API Pix do Banco Central): QR Code na tela e baixa automática.",
            )
        }
    }

    async fn check(&self, call: &Call<'_>) -> Result<String, ProviderError> {
        let cert = call
            .file("certificado")
            .ok_or_else(|| ProviderError::Auth("falta o certificado".into()))?;
        let subject =
            Pkcs12Pem::parse(&cert, &call.secret("senha_certificado").unwrap_or_default())?.subject;
        let http = self.http(call)?;
        self.token(call, &http).await?;
        Ok(format!("Conta Efí conectada. Certificado: {subject}"))
    }

    async fn run(&self, call: &Call<'_>, job: &Job) -> Outcome {
        match job.kind.as_str() {
            "pix.create" => match self.create(call, job).await {
                Ok(v) => Outcome::Done(v),
                Err(e) => Outcome::Error(e),
            },
            "pix.poll" => {
                let txid = match pix::provider_ref(job) {
                    Ok(t) => t,
                    Err(e) => return Outcome::Error(e),
                };
                match self
                    .send(call, reqwest::Method::GET, &format!("/v2/cob/{txid}"), None)
                    .await
                {
                    Ok(b) => {
                        let out = poll_outcome(&b, job);
                        if out == Outcome::Done(json!({ "status": "expired" })) {
                            let _ = self.cancel(call, &txid).await;
                        }
                        out
                    }
                    Err(e) => Outcome::Error(e),
                }
            }
            "pix.cancel" => match pix::provider_ref(job) {
                Ok(t) => match self.cancel(call, &t).await {
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

    #[test]
    fn cob_body_follows_central_bank_format() {
        let req = CreateReq {
            txid: "NUC0123456789abcdef0123456789abcd".into(),
            amount_cents: 1005,
            description: "Venda nº 3".into(),
            expires_secs: 3600,
            payer: Some(Payer {
                name: "Padaria".into(),
                document: Some("12.345.678/0001-95".into()),
                email: None,
            }),
        };
        let b = cob_body(&req, "chave@loja.com");
        assert_eq!(b["valor"]["original"], "10.05");
        assert_eq!(b["calendario"]["expiracao"], 3600);
        assert_eq!(b["devedor"]["cnpj"], "12345678000195");
        assert_eq!(b["chave"], "chave@loja.com");
    }

    #[test]
    fn status_mapping() {
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
        match poll_outcome(
            &json!({ "status": "CONCLUIDA", "pix": [{ "horario": "2026-11-04T13:00:00.000Z" }] }),
            &job,
        ) {
            Outcome::Done(v) => assert_eq!(v["paid_at"], "2026-11-04T13:00:00.000Z"),
            o => panic!("{o:?}"),
        }
        assert!(matches!(
            poll_outcome(&json!({ "status": "ATIVA" }), &job),
            Outcome::Wait { .. }
        ));
        assert_eq!(
            poll_outcome(&json!({ "status": REMOVED }), &job),
            Outcome::Done(json!({ "status": "canceled" }))
        );
    }
}
