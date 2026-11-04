use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::modules::double_option;

pub const STATUSES: [&str; 6] = [
    "aberta",
    "em_andamento",
    "aguardando",
    "concluida",
    "faturada",
    "cancelada",
];
/// status que a equipe move livremente (faturada/cancelada só por ação própria)
pub const WORK_STATUSES: [&str; 4] = ["aberta", "em_andamento", "aguardando", "concluida"];

#[derive(Debug, Clone, Serialize)]
pub struct OsItem {
    pub id: String,
    pub product_id: String,
    pub description: String,
    pub is_service: bool,
    pub qty: Decimal,
    pub unit_price: Decimal,
    pub discount: Decimal,
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct OsEvent {
    pub id: String,
    pub kind: String,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub note: Option<String>,
    pub user_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceOrder {
    pub id: String,
    pub number: i64,
    pub customer_id: String,
    pub customer_name: Option<String>,
    pub customer_phone: Option<String>,
    pub status: String,
    pub equipment: Option<String>,
    pub reported: String,
    pub diagnosis: Option<String>,
    pub technician_id: Option<String>,
    pub technician_name: Option<String>,
    pub scheduled_date: Option<String>,
    pub due_date: Option<String>,
    pub subtotal: Decimal,
    pub discount: Decimal,
    pub total: Decimal,
    pub sale_id: Option<String>,
    pub sale_number: Option<i64>,
    pub finished_at: Option<String>,
    pub canceled_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub item_count: i64,
    /// só no detalhe
    pub items: Option<Vec<OsItem>>,
    pub events: Option<Vec<OsEvent>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsItemInput {
    pub product_id: String,
    /// padrão: nome do produto
    pub description: Option<String>,
    pub qty: Decimal,
    /// padrão: preço do cadastro
    pub unit_price: Option<Decimal>,
    pub discount: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct OsInput {
    pub customer_id: String,
    pub equipment: Option<String>,
    pub reported: String,
    pub technician_id: Option<String>,
    pub scheduled_date: Option<String>,
    pub due_date: Option<String>,
    #[serde(default)]
    pub items: Vec<OsItemInput>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OsPatch {
    #[serde(default, deserialize_with = "double_option")]
    pub equipment: Option<Option<String>>,
    pub reported: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub diagnosis: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub technician_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub scheduled_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub due_date: Option<Option<String>>,
    pub discount: Option<Decimal>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OsQuery {
    pub status: Option<String>,
    pub customer_id: Option<String>,
    pub technician_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InvoiceInput {
    pub payment_method: String,
    pub seller_id: Option<String>,
    pub tz_offset_minutes: Option<i32>,
}
