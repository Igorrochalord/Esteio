use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::modules::double_option;

pub const ACCOUNT_KINDS: [&str; 4] = ["caixa", "corrente", "poupanca", "carteira"];
pub const DRE_GROUPS: [&str; 11] = [
    "receita_vendas",
    "outras_receitas",
    "deducoes",
    "custos",
    "despesas_operacionais",
    "despesas_pessoal",
    "despesas_financeiras",
    "receitas_financeiras",
    "impostos",
    "investimentos",
    // compra de mercadoria para revenda: vira estoque; o custo entra no CMV quando vende
    "compras_estoque",
];

#[derive(Debug, Clone, Serialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub bank_name: Option<String>,
    pub agency: Option<String>,
    pub account_number: Option<String>,
    pub initial_balance: Decimal,
    pub opening_date: String,
    pub is_default: bool,
    pub active: bool,
    pub balance: Decimal,
    pub unreconciled: i64,
}

#[derive(Debug, Deserialize)]
pub struct AccountInput {
    pub name: String,
    pub kind: String,
    pub bank_name: Option<String>,
    pub agency: Option<String>,
    pub account_number: Option<String>,
    pub initial_balance: Option<Decimal>,
    /// AAAA-MM-DD; padrão hoje
    pub opening_date: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
pub struct AccountPatch {
    pub name: Option<String>,
    pub kind: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub bank_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub agency: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub account_number: Option<Option<String>>,
    pub initial_balance: Option<Decimal>,
    pub is_default: Option<bool>,
    pub active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub dre_group: String,
    pub system_key: Option<String>,
    pub active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CategoryInput {
    pub name: String,
    pub kind: String,
    pub dre_group: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct CategoryPatch {
    pub name: Option<String>,
    pub dre_group: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub description: String,
    pub category_id: String,
    pub category_name: Option<String>,
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
    pub counterparty: Option<String>,
    pub amount: Decimal,
    pub issue_date: String,
    pub due_date: String,
    pub status: String,
    pub paid_date: Option<String>,
    pub paid_amount: Option<Decimal>,
    pub bank_account_id: Option<String>,
    pub bank_account_name: Option<String>,
    pub payment_method: Option<String>,
    pub document: Option<String>,
    pub installment: Option<i64>,
    pub installments: Option<i64>,
    pub group_id: Option<String>,
    pub sale_id: Option<String>,
    pub sale_number: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct EntryQuery {
    pub kind: Option<String>,
    /// aberto | pago | cancelado
    pub status: Option<String>,
    /// vencimento entre (AAAA-MM-DD, inclusivo)
    pub due_from: Option<String>,
    pub due_to: Option<String>,
    pub customer_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EntryInput {
    pub kind: String,
    pub description: String,
    pub category_id: String,
    pub customer_id: Option<String>,
    pub counterparty: Option<String>,
    /// valor total (parcelado) ou de cada lançamento (recorrente)
    pub amount: Decimal,
    pub issue_date: Option<String>,
    /// vencimento da 1ª parcela/ocorrência
    pub due_date: String,
    /// divide o valor em N parcelas mensais
    pub installments: Option<u32>,
    /// repete o mesmo valor por N meses
    pub repeat_months: Option<u32>,
    pub document: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct EntryPatch {
    pub description: Option<String>,
    pub category_id: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub customer_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub counterparty: Option<Option<String>>,
    pub amount: Option<Decimal>,
    pub due_date: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub document: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct PayInput {
    /// AAAA-MM-DD
    pub date: String,
    /// valor efetivamente pago/recebido (juros, desconto); padrão = valor do título
    pub amount: Option<Decimal>,
    pub bank_account_id: Option<String>,
    pub payment_method: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Transaction {
    pub id: String,
    pub bank_account_id: String,
    pub bank_account_name: String,
    pub date: String,
    pub amount: Decimal,
    pub balance_after: Decimal,
    pub description: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub entry_id: Option<String>,
    pub sale_id: Option<String>,
    pub transfer_group: Option<String>,
    pub reconciled: bool,
    /// manual = lançamento avulso ou transferência (pode excluir)
    pub origin: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct TxQuery {
    pub bank_account_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TxInput {
    pub bank_account_id: String,
    pub date: String,
    /// entrada | saida
    pub kind: String,
    pub amount: Decimal,
    pub description: String,
    pub category_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TransferInput {
    pub from_account_id: String,
    pub to_account_id: String,
    pub date: String,
    pub amount: Decimal,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OfxLine {
    pub fitid: String,
    pub date: String,
    pub amount: Decimal,
    pub memo: String,
    /// novo | conciliar | ja_importado
    pub status: String,
    pub match_id: Option<String>,
    pub match_description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OfxPreview {
    pub bank_id: Option<String>,
    pub account_id: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub ledger_balance: Option<Decimal>,
    pub lines: Vec<OfxLine>,
}

#[derive(Debug, Deserialize)]
pub struct OfxImportRequest {
    pub bank_account_id: String,
    pub path: String,
    /// cria lançamentos para as linhas sem correspondência
    pub create_new: bool,
    pub income_category_id: Option<String>,
    pub expense_category_id: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct OfxImportResult {
    pub reconciled: usize,
    pub created: usize,
    pub skipped: usize,
}

#[derive(Debug, Serialize)]
pub struct FinanceSummary {
    pub total_balance: Decimal,
    pub receivable_overdue: Decimal,
    pub receivable_overdue_count: i64,
    pub receivable_next7: Decimal,
    pub payable_overdue: Decimal,
    pub payable_overdue_count: i64,
    pub payable_next7: Decimal,
    pub payable_today: Decimal,
}
