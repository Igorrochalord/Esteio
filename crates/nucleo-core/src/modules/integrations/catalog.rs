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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    /// texto comum, guardado na configuração
    Text,
    /// segredo (token, senha): vai cifrado e nunca volta para a tela
    Secret,
    /// arquivo (ex.: certificado .p12/.pfx): base64, cifrado como os segredos
    File,
}

#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
    pub required: bool,
    pub help: Option<&'static str>,
}

impl Field {
    /// Segredos e arquivos vão para o cofre; texto vai para a configuração.
    pub fn is_sealed(&self) -> bool {
        matches!(self.kind, FieldKind::Secret | FieldKind::File)
    }

    pub const fn text(key: &'static str, label: &'static str, required: bool) -> Self {
        Self {
            key,
            label,
            kind: FieldKind::Text,
            required,
            help: None,
        }
    }

    pub const fn secret(key: &'static str, label: &'static str, required: bool) -> Self {
        Self {
            key,
            label,
            kind: FieldKind::Secret,
            required,
            help: None,
        }
    }

    pub const fn file(key: &'static str, label: &'static str, required: bool) -> Self {
        Self {
            key,
            label,
            kind: FieldKind::File,
            required,
            help: None,
        }
    }

    pub const fn help(mut self, help: &'static str) -> Self {
        self.help = Some(help);
        self
    }
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
    /// o que o provedor sabe fazer: "pix", "marketplace", "nfse"
    pub capabilities: Vec<&'static str>,
    /// só funciona no ambiente de teste (ex.: Simulador)
    pub sandbox_only: bool,
    /// autorização pelo navegador (OAuth): a tela mostra "Autorizar" e pede o código
    pub oauth: bool,
}

impl ProviderInfo {
    /// Informações mínimas; os campos opcionais começam vazios.
    pub fn new(
        id: &'static str,
        name: &'static str,
        kind: Kind,
        description: &'static str,
    ) -> Self {
        Self {
            id,
            name,
            kind,
            description,
            fields: vec![],
            available: true,
            phase: None,
            capabilities: vec![],
            sandbox_only: false,
            oauth: false,
        }
    }
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
            available: false,
            phase: Some(phase),
            ..ProviderInfo::new(id, name, kind, description)
        })
        .collect()
}
