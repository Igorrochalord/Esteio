use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::modules::double_option;

pub const UNITS: [&str; 9] = ["un", "kg", "g", "l", "ml", "cx", "pct", "m", "h"];
pub const MANUAL_KINDS: [&str; 3] = ["entrada", "saida", "ajuste"];

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProductRow {
    pub id: String,
    pub code: String,
    pub barcode: Option<String>,
    pub name: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub unit: String,
    pub price_cents: i64,
    pub cost_cents: i64,
    pub track_stock: bool,
    pub stock_milli: i64,
    pub min_stock_milli: i64,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Produto como a interface vê. `cost` só vai para quem pode editar produtos.
#[derive(Debug, Clone, Serialize)]
pub struct Product {
    pub id: String,
    pub code: String,
    pub barcode: Option<String>,
    pub name: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub unit: String,
    pub price: Decimal,
    pub cost: Option<Decimal>,
    pub track_stock: bool,
    pub stock: Decimal,
    pub min_stock: Decimal,
    pub below_min: bool,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ProductInput {
    /// vazio = gerado automaticamente
    #[serde(default)]
    pub code: String,
    pub barcode: Option<String>,
    pub name: String,
    pub category_id: Option<String>,
    pub unit: Option<String>,
    pub price: Decimal,
    pub cost: Option<Decimal>,
    pub track_stock: Option<bool>,
    pub min_stock: Option<Decimal>,
    /// lança uma entrada inicial de estoque
    pub initial_stock: Option<Decimal>,
}

/// Edição parcial (uma célula da planilha = um campo).
#[derive(Debug, Default, Deserialize)]
pub struct ProductPatch {
    pub code: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub barcode: Option<Option<String>>,
    pub name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub category_id: Option<Option<String>>,
    pub unit: Option<String>,
    pub price: Option<Decimal>,
    pub cost: Option<Decimal>,
    pub track_stock: Option<bool>,
    pub min_stock: Option<Decimal>,
    pub active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub product_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct StockInput {
    pub product_id: String,
    /// entrada (soma), saida (subtrai) ou ajuste (`qty` = saldo contado)
    pub kind: String,
    pub qty: Decimal,
    /// custo unitário da entrada (atualiza o custo do produto)
    pub unit_cost: Option<Decimal>,
    pub note: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct MovementRow {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub kind: String,
    pub qty_milli: i64,
    pub balance_milli: i64,
    pub unit_cost_cents: Option<i64>,
    pub note: Option<String>,
    pub sale_id: Option<String>,
    pub user_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct Movement {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub kind: String,
    pub qty: Decimal,
    pub balance: Decimal,
    pub unit_cost: Option<Decimal>,
    pub note: Option<String>,
    pub sale_id: Option<String>,
    pub user_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct MovementQuery {
    pub product_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
}
