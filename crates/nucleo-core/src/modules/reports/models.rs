use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Período dos relatórios. `from`/`to` são instantes (o front manda a meia-noite local em
/// RFC 3339) ou datas `AAAA-MM-DD`; `to` é exclusivo. `tz_offset_minutes` = minutos a somar
/// ao UTC para obter a hora local (Brasília = -180), usado para agrupar por dia/semana/mês.
#[derive(Debug, Default, Deserialize)]
pub struct ReportQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    #[serde(default)]
    pub tz_offset_minutes: i32,
    /// day | week | month
    pub group: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Default, Serialize)]
pub struct SalesTotals {
    pub count: i64,
    pub gross: Decimal,
    pub discount: Decimal,
    pub total: Decimal,
    pub cost: Decimal,
    pub profit: Decimal,
    /// lucro / total, em %
    pub margin_pct: Decimal,
    pub avg_ticket: Decimal,
    pub items_qty: Decimal,
    pub canceled_count: i64,
    pub canceled_total: Decimal,
}

#[derive(Debug, Serialize)]
pub struct PeriodRow {
    /// chave do período: AAAA-MM-DD, AAAA-Wss ou AAAA-MM (hora local)
    pub period: String,
    pub count: i64,
    pub total: Decimal,
    pub cost: Decimal,
    pub profit: Decimal,
}

#[derive(Debug, Serialize)]
pub struct PaymentRow {
    pub method: String,
    pub count: i64,
    pub total: Decimal,
    pub share_pct: Decimal,
}

#[derive(Debug, Serialize)]
pub struct SalesSummary {
    pub totals: SalesTotals,
    pub series: Vec<PeriodRow>,
    pub by_payment: Vec<PaymentRow>,
    /// vendas por hora do dia (0–23, hora local)
    pub by_hour: Vec<PeriodRow>,
}

#[derive(Debug, Serialize)]
pub struct ProductRank {
    pub product_id: String,
    pub code: Option<String>,
    pub name: String,
    pub category: Option<String>,
    pub qty: Decimal,
    pub revenue: Decimal,
    pub cost: Decimal,
    pub profit: Decimal,
    pub margin_pct: Decimal,
    pub share_pct: Decimal,
    pub cumulative_pct: Decimal,
    /// curva ABC: A até 80% do faturamento, B até 95%, C o resto
    pub abc: String,
}

#[derive(Debug, Serialize)]
pub struct CategoryRow {
    pub category: String,
    pub qty: Decimal,
    pub revenue: Decimal,
    pub cost: Decimal,
    pub profit: Decimal,
    pub margin_pct: Decimal,
}

#[derive(Debug, Serialize)]
pub struct ProductsReport {
    pub products: Vec<ProductRank>,
    pub categories: Vec<CategoryRow>,
}

#[derive(Debug, Serialize)]
pub struct StockRow {
    pub product_id: String,
    pub code: String,
    pub name: String,
    pub category: Option<String>,
    pub unit: String,
    pub stock: Decimal,
    pub min_stock: Decimal,
    pub below_min: bool,
    pub cost_value: Decimal,
    pub price_value: Decimal,
    /// quantidade vendida no período
    pub sold_qty: Decimal,
    /// dias de estoque no ritmo de venda do período (None = sem venda)
    pub days_of_cover: Option<Decimal>,
}

#[derive(Debug, Serialize)]
pub struct StockReport {
    pub rows: Vec<StockRow>,
    pub total_cost_value: Decimal,
    pub total_price_value: Decimal,
    pub below_min_count: i64,
    pub out_of_stock_count: i64,
}

#[derive(Debug, Serialize)]
pub struct StateRow {
    pub state: String,
    pub customers: i64,
    pub count: i64,
    pub total: Decimal,
}

#[derive(Debug, Serialize)]
pub struct CityRow {
    pub city: String,
    pub state: Option<String>,
    pub customers: i64,
    pub count: i64,
    pub total: Decimal,
}

#[derive(Debug, Serialize)]
pub struct CustomerRank {
    pub customer_id: String,
    pub name: String,
    pub city: Option<String>,
    pub state: Option<String>,
    pub count: i64,
    pub total: Decimal,
    pub avg_ticket: Decimal,
    pub last_purchase: String,
}

#[derive(Debug, Serialize)]
pub struct CustomersReport {
    pub by_state: Vec<StateRow>,
    pub by_city: Vec<CityRow>,
    pub top_customers: Vec<CustomerRank>,
    /// vendas sem cliente identificado
    pub anonymous_count: i64,
    pub anonymous_total: Decimal,
    /// clientes com compra no período, sem UF cadastrada
    pub without_state: i64,
}

#[derive(Debug, Serialize)]
pub struct SellerRow {
    pub seller_id: String,
    pub name: String,
    pub count: i64,
    pub total: Decimal,
    pub avg_ticket: Decimal,
    pub profit: Decimal,
    pub margin_pct: Decimal,
    pub customers_served: i64,
    pub items_qty: Decimal,
    pub canceled_count: i64,
    pub share_pct: Decimal,
}
