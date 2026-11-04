use rust_decimal::Decimal;
use serde::Serialize;

/// Dados de quem paga, quando a venda tem cliente (alguns provedores exigem).
#[derive(Debug, Clone, Serialize, serde::Deserialize, Default)]
pub struct Payer {
    pub name: String,
    pub document: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChargeRow {
    pub id: String,
    pub sale_id: String,
    pub sale_number: i64,
    pub entry_id: String,
    pub integration_id: String,
    pub provider: String,
    pub amount_cents: i64,
    pub status: String,
    pub txid: String,
    pub provider_ref: Option<String>,
    pub copy_paste: Option<String>,
    pub expires_at: Option<String>,
    pub paid_at: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
}

/// Cobrança como a tela vê. `qr_svg` é o QR desenhado aqui (sem internet).
#[derive(Debug, Clone, Serialize)]
pub struct PixCharge {
    pub id: String,
    pub sale_id: String,
    pub sale_number: i64,
    pub provider: String,
    pub provider_name: String,
    pub amount: Decimal,
    /// gerando, aguardando, pago, expirado, cancelado, erro
    pub status: String,
    pub copy_paste: Option<String>,
    pub qr_svg: Option<String>,
    pub expires_at: Option<String>,
    pub paid_at: Option<String>,
    pub error: Option<String>,
    /// ambiente de teste: o pagamento não é real
    pub sandbox: bool,
    pub created_at: String,
}

/// Provedor de PIX pronto para uso na venda.
#[derive(Debug, Clone, Serialize)]
pub struct PixProviderOption {
    pub id: String,
    pub name: String,
    pub sandbox: bool,
    /// a última verificação de conexão deu certo
    pub checked: bool,
}
