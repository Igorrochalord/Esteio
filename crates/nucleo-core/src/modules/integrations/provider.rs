//! Contrato de um provedor de API. Cada integração (PIX, NFS-e, marketplace) implementa
//! `Provider` e é registrada no `Core` (`Core::register_provider`).

use async_trait::async_trait;
use serde_json::Value;

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
}

impl Call<'_> {
    pub fn config_str(&self, key: &str) -> Option<&str> {
        self.config.get(key).and_then(Value::as_str)
    }

    pub fn secret(&self, key: &str) -> Option<&str> {
        self.secrets.get(key).and_then(Value::as_str)
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
    /// ainda não (ex.: PIX aguardando pagamento): volta daqui a `secs`, sem contar como falha
    Wait { secs: i64, note: Option<String> },
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
