//! Contrato de um provedor de API. Cada integração (PIX, NFS-e, marketplace) implementa
//! `Provider` e é registrada no `Core` (`Core::register_provider`).

use std::sync::Mutex;

use async_trait::async_trait;
use base64::Engine;
use serde_json::{Map, Value};

use crate::db::{Conn, Installation};
use crate::error::CoreResult;

use super::catalog::ProviderInfo;
use super::http::Http;
use super::models::Job;

/// Tudo o que uma chamada precisa: cliente HTTP, ambiente e credenciais já decifradas.
pub struct Call<'a> {
    pub http: &'a Http,
    pub production: bool,
    pub config: &'a Value,
    pub secrets: &'a Value,
    /// segredos novos (ex.: token renovado), gravados pelo serviço depois da chamada
    updated: Mutex<Option<Map<String, Value>>>,
}

impl<'a> Call<'a> {
    pub fn new(http: &'a Http, production: bool, config: &'a Value, secrets: &'a Value) -> Self {
        Self {
            http,
            production,
            config,
            secrets,
            updated: Mutex::new(None),
        }
    }

    pub fn config_str(&self, key: &str) -> Option<&str> {
        self.config
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
    }

    /// Segredo atual (o renovado nesta chamada, se houver).
    pub fn secret(&self, key: &str) -> Option<String> {
        if let Some(v) = self
            .updated
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(|m| m.get(key))
        {
            return v.as_str().map(str::to_owned);
        }
        self.secrets
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    /// Arquivo enviado na tela (base64 → bytes).
    pub fn file(&self, key: &str) -> Option<Vec<u8>> {
        let b64 = self.secret(key)?;
        base64::engine::general_purpose::STANDARD.decode(b64).ok()
    }

    /// Troca/acrescenta segredos (ex.: access/refresh token). O serviço cifra e grava.
    pub fn set_secrets(&self, values: Map<String, Value>) {
        let mut guard = self.updated.lock().unwrap_or_else(|e| e.into_inner());
        let merged =
            guard.get_or_insert_with(|| self.secrets.as_object().cloned().unwrap_or_default());
        for (k, v) in values {
            merged.insert(k, v);
        }
    }

    /// Segredos alterados nesta chamada (todos, já mesclados), se houve alteração.
    pub fn take_updated(&self) -> Option<Map<String, Value>> {
        self.updated
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// sem internet ou servidor fora do ar: tenta de novo depois
    Offline,
    /// credencial recusada (401/403)
    Auth(String),
    /// a API recusou o pedido (dados inválidos etc.): não adianta repetir igual
    Rejected(String),
    /// erro temporário do lado da API (5xx, 429)
    Unavailable(String),
}

impl ProviderError {
    /// Mensagem para o usuário final.
    pub fn message(&self) -> String {
        match self {
            Self::Offline => "Sem conexão com a internet ou o serviço não respondeu.".into(),
            Self::Auth(_) => {
                "As credenciais foram recusadas. Confira os dados da integração.".into()
            }
            Self::Rejected(d) => format!("O serviço recusou o pedido: {d}"),
            Self::Unavailable(_) => {
                "O serviço está instável no momento. Vamos tentar de novo.".into()
            }
        }
    }

    /// Vale a pena repetir mais tarde?
    pub fn transient(&self) -> bool {
        matches!(self, Self::Offline | Self::Unavailable(_))
    }
}

/// Resultado de um trabalho da fila.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// terminou; `on_done` aplica o efeito local na mesma transação
    Done(Value),
    /// ainda não (ex.: PIX aguardando pagamento): volta daqui a `secs`, sem contar como falha.
    /// `payload` troca os dados do trabalho (ex.: cursor da sincronização de pedidos).
    Wait {
        secs: i64,
        note: Option<String>,
        payload: Option<Value>,
    },
    /// falhou; o serviço decide entre repetir (transitório) e desistir
    Error(ProviderError),
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn info(&self) -> ProviderInfo;

    /// Chamada leve que confirma as credenciais. Devolve um detalhe para a tela
    /// (ex.: "Conta: Padaria Pão Dourado").
    async fn check(&self, call: &Call<'_>) -> Result<String, ProviderError>;

    async fn run(&self, call: &Call<'_>, job: &Job) -> Outcome;

    /// Endereço de autorização no navegador (provedores com `info().oauth`).
    fn authorize_url(&self, _call: &Call<'_>) -> Option<String> {
        None
    }

    /// Troca o código colado pela pessoa por tokens (`call.set_secrets`). Devolve um detalhe.
    async fn exchange_code(&self, _call: &Call<'_>, _code: &str) -> Result<String, ProviderError> {
        Err(ProviderError::Rejected(
            "este serviço não usa autorização pelo navegador".into(),
        ))
    }

    /// Efeito local de um trabalho concluído (ex.: marcar a venda como paga).
    async fn on_done(
        &self,
        _conn: &mut Conn,
        _inst: &Installation,
        _job: &Job,
        _result: &Value,
    ) -> CoreResult<()> {
        Ok(())
    }
}
