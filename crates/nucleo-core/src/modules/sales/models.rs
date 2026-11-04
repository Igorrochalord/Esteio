use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const PAYMENT_METHODS: [&str; 6] = ["dinheiro", "pix", "credito", "debito", "fiado", "outro"];

#[derive(Debug, Deserialize)]
pub struct SaleItemInput {
    pub product_id: String,
    pub qty: Decimal,
    /// preço unitário; ausente = preço do cadastro
    pub unit_price: Option<Decimal>,
    pub discount: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct SaleInput {
    pub customer_id: Option<String>,
    /// vendedor; ausente = quem está registrando
    pub seller_id: Option<String>,
    pub payment_method: String,
    /// desconto no total da venda
    pub discount: Option<Decimal>,
    pub notes: Option<String>,
    /// data da venda (só para quem gerencia vendas; padrão = agora)
    pub sold_at: Option<String>,
    /// fuso da tela (minutos a somar ao UTC); define a data local no financeiro. Padrão: Brasília
    pub tz_offset_minutes: Option<i32>,
    pub items: Vec<SaleItemInput>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct SaleRow {
    pub id: String,
    pub number: i64,
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
    pub seller_id: String,
    pub seller_name: Option<String>,
    pub status: String,
    pub payment_method: String,
    pub subtotal_cents: i64,
    pub discount_cents: i64,
    pub total_cents: i64,
    pub cost_total_cents: i64,
    pub notes: Option<String>,
    pub sold_at: String,
    pub canceled_at: Option<String>,
    pub cancel_reason: Option<String>,
    pub item_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct SaleItemRow {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub qty_milli: i64,
    pub unit_price_cents: i64,
    pub unit_cost_cents: i64,
    pub discount_cents: i64,
    pub total_cents: i64,
}

#[derive(Debug, Serialize)]
pub struct SaleItem {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub qty: Decimal,
    pub unit_price: Decimal,
    pub discount: Decimal,
    pub total: Decimal,
}

/// Venda como a interface vê. Custo e lucro só para quem vê relatórios.
#[derive(Debug, Serialize)]
pub struct Sale {
    pub id: String,
    pub number: i64,
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
    pub seller_id: String,
    pub seller_name: Option<String>,
    pub status: String,
    pub payment_method: String,
    pub subtotal: Decimal,
    pub discount: Decimal,
    pub total: Decimal,
    pub cost_total: Option<Decimal>,
    pub profit: Option<Decimal>,
    pub notes: Option<String>,
    pub sold_at: String,
    pub canceled_at: Option<String>,
    pub cancel_reason: Option<String>,
    pub item_count: i64,
    /// só no detalhe (`get`)
    pub items: Option<Vec<SaleItem>>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SaleQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub status: Option<String>,
    pub customer_id: Option<String>,
    pub seller_id: Option<String>,
    pub limit: Option<i64>,
}
