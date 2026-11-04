//! Catálogo de provedores. Os implementados vêm do próprio `Provider::info()`;
//! os planejados aparecem na tela como "em breve".

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// cobrança PIX na venda
    Pix,
    /// nota fiscal de serviço
    Nfse,
    /// pedidos de marketplaces e lojas virtuais
    Marketplace,
    /// usado nos testes
    Other,
}

#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    /// segredo: vai cifrado e nunca volta para a tela
    pub secret: bool,
    pub required: bool,
    pub help: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: Kind,
    pub description: &'static str,
    pub fields: Vec<Field>,
    /// `false` = planejado; aparece como "em breve"
    pub available: bool,
    /// fase do roteiro em que entra (para os planejados)
    pub phase: Option<&'static str>,
}

/// Provedores previstos no roteiro (docs/roteiro.md, fase 6). O provedor de PIX e os
/// marketplaces definitivos saem da pesquisa de cada subfase.
const PLANNED: &[(&str, &str, Kind, &str, &str)] = &[
    (
        "pix",
        "PIX na venda",
        Kind::Pix,
        "Gera o QR Code PIX na tela da venda e dá baixa sozinho quando o pagamento cai.",
        "6b",
    ),
    (
        "nfse_nacional",
        "NFS-e (Emissor Nacional)",
        Kind::Nfse,
        "Emite a nota fiscal de serviço pelo padrão nacional, com certificado A1.",
        "6c",
    ),
    (
        "marketplaces",
        "Marketplaces e loja virtual",
        Kind::Marketplace,
        "Traz os pedidos das lojas on-line como vendas e baixa o estoque.",
        "6d",
    ),
];

pub fn planned_infos() -> Vec<ProviderInfo> {
    PLANNED
        .iter()
        .map(|&(id, name, kind, description, phase)| ProviderInfo {
            id,
            name,
            kind,
            description,
            fields: vec![],
            available: false,
            phase: Some(phase),
        })
        .collect()
}
