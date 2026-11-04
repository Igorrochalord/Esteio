export type Role = "owner" | "admin" | "user";
export type Level = "read" | "write" | "manage";
export type Module =
  | "settings"
  | "users"
  | "audit"
  | "backup"
  | "products"
  | "customers"
  | "sales"
  | "reports"
  | "finance"
  | "service_orders";

export interface User {
  id: string;
  name: string;
  email: string;
  role: Role;
  is_active: boolean;
  /** autocadastro aguardando aprovação */
  pending: boolean;
  last_login_at: string | null;
  created_at: string;
}

export interface Branding {
  name: string;
  primary_color: string;
  secondary_color: string;
  logo_url: string | null;
}

export interface Me {
  user: User;
  company: Branding & { id: string };
  permissions: Partial<Record<Module, Level>>;
  /** hoje: acesso completo de lançamento; no futuro, o plano contratado */
  plan: { code: string; label: string; unlimited: boolean; note: string };
}

export interface Company {
  id: string;
  name: string;
  cnpj: string | null;
}

export interface AuditEntry {
  id: string;
  user_id: string | null;
  user_name: string | null;
  action: string;
  entity: string;
  entity_id: string | null;
  before: Record<string, unknown> | null;
  after: Record<string, unknown> | null;
  created_at: string;
}

export interface Page<T> {
  items: T[];
  total: number;
  limit: number;
  offset: number;
}

export interface BackupInfo {
  name: string;
  path: string;
  size_bytes: number;
  created_at: string;
  kind: "auto" | "manual" | "pre-restore";
}

/** Valores decimais chegam como texto ("12.50") para não perder centavos. */
export type Dec = string;

export interface Product {
  id: string;
  code: string;
  barcode: string | null;
  name: string;
  category_id: string | null;
  category_name: string | null;
  unit: string;
  price: Dec;
  /** só para quem pode editar produtos */
  cost: Dec | null;
  track_stock: boolean;
  stock: Dec;
  min_stock: Dec;
  below_min: boolean;
  active: boolean;
  created_at: string;
  updated_at: string;
}

export interface Category {
  id: string;
  name: string;
  product_count: number;
}

export type StockKind = "entrada" | "saida" | "ajuste" | "venda" | "estorno_venda";

export interface Movement {
  id: string;
  product_id: string;
  product_name: string;
  kind: StockKind;
  qty: Dec;
  balance: Dec;
  unit_cost: Dec | null;
  note: string | null;
  sale_id: string | null;
  user_name: string | null;
  created_at: string;
}

export interface Customer {
  id: string;
  name: string;
  document: string | null;
  phone: string | null;
  email: string | null;
  city: string | null;
  state: string | null;
  notes: string | null;
  active: boolean;
  created_at: string;
  updated_at: string;
}

export type PaymentMethod = "dinheiro" | "pix" | "credito" | "debito" | "fiado" | "outro";

export interface SaleItem {
  id: string;
  product_id: string;
  product_name: string;
  qty: Dec;
  unit_price: Dec;
  discount: Dec;
  total: Dec;
}

export interface Sale {
  id: string;
  number: number;
  customer_id: string | null;
  customer_name: string | null;
  seller_id: string;
  seller_name: string | null;
  status: "concluida" | "cancelada";
  payment_method: PaymentMethod;
  subtotal: Dec;
  discount: Dec;
  total: Dec;
  cost_total: Dec | null;
  profit: Dec | null;
  notes: string | null;
  sold_at: string;
  canceled_at: string | null;
  cancel_reason: string | null;
  item_count: number;
  items: SaleItem[] | null;
}

export interface SellerOption {
  id: string;
  name: string;
}

export interface ReportQuery {
  from?: string;
  to?: string;
  tz_offset_minutes: number;
  group?: "day" | "week" | "month";
  limit?: number;
}

export interface PeriodRow {
  period: string;
  count: number;
  total: Dec;
  cost: Dec;
  profit: Dec;
}

export interface SalesSummary {
  totals: {
    count: number;
    gross: Dec;
    discount: Dec;
    total: Dec;
    cost: Dec;
    profit: Dec;
    margin_pct: Dec;
    avg_ticket: Dec;
    items_qty: Dec;
    canceled_count: number;
    canceled_total: Dec;
  };
  series: PeriodRow[];
  by_payment: { method: PaymentMethod; count: number; total: Dec; share_pct: Dec }[];
  by_hour: PeriodRow[];
}

export interface ProductRank {
  product_id: string;
  code: string | null;
  name: string;
  category: string | null;
  unit: string;
  qty: Dec;
  revenue: Dec;
  cost: Dec;
  profit: Dec;
  margin_pct: Dec;
  share_pct: Dec;
  cumulative_pct: Dec;
  abc: "A" | "B" | "C";
}

export interface ProductsReport {
  products: ProductRank[];
  categories: { category: string; qty: Dec; revenue: Dec; cost: Dec; profit: Dec; margin_pct: Dec }[];
}

export interface StockRow {
  product_id: string;
  code: string;
  name: string;
  category: string | null;
  unit: string;
  stock: Dec;
  min_stock: Dec;
  below_min: boolean;
  cost_value: Dec;
  price_value: Dec;
  sold_qty: Dec;
  days_of_cover: Dec | null;
}

export interface StockReport {
  rows: StockRow[];
  total_cost_value: Dec;
  total_price_value: Dec;
  below_min_count: number;
  out_of_stock_count: number;
}

export interface CustomersReport {
  by_state: { state: string; customers: number; count: number; total: Dec }[];
  by_city: { city: string; state: string | null; customers: number; count: number; total: Dec }[];
  top_customers: {
    customer_id: string;
    name: string;
    city: string | null;
    state: string | null;
    count: number;
    total: Dec;
    avg_ticket: Dec;
    last_purchase: string;
  }[];
  anonymous_count: number;
  anonymous_total: Dec;
  without_state: number;
}

export interface SellerRow {
  seller_id: string;
  name: string;
  count: number;
  total: Dec;
  avg_ticket: Dec;
  profit: Dec;
  margin_pct: Dec;
  customers_served: number;
  items_qty: Dec;
  canceled_count: number;
  share_pct: Dec;
}

export interface SheetColumn {
  label: string;
  kind: "text" | "number" | "money" | "percent" | "date" | "datetime";
}

export interface SheetInput {
  name: string;
  columns: SheetColumn[];
  rows: (string | number | boolean | null)[][];
}

export type ImportField =
  | "code"
  | "name"
  | "barcode"
  | "category"
  | "unit"
  | "price"
  | "cost"
  | "stock"
  | "min_stock";

export interface ImportPreview {
  headers: string[];
  sample: string[][];
  total_rows: number;
  suggested: Partial<Record<ImportField, number>>;
}

export interface ImportResult {
  created: number;
  updated: number;
  skipped: number;
  errors: { row: number; message: string }[];
}

export type PlanCode = "gratuito" | "starter" | "platinum" | "infinite";

export interface PlanLimits {
  bank_accounts: number | null;
  nfse_per_month: number | null;
  extra_cnpjs: number | null;
  nfse: boolean;
  service_orders: boolean;
  bank_reconciliation: boolean;
  advanced_reports: boolean;
}

export interface PlanItem {
  plan: PlanCode;
  price: Dec;
  limits: PlanLimits;
}

// ------------------------------------------------------------------ financeiro

export type AccountKind = "caixa" | "corrente" | "poupanca" | "carteira";

export interface BankAccount {
  id: string;
  name: string;
  kind: AccountKind;
  bank_name: string | null;
  agency: string | null;
  account_number: string | null;
  initial_balance: Dec;
  opening_date: string;
  is_default: boolean;
  active: boolean;
  balance: Dec;
  unreconciled: number;
}

export interface FinCategory {
  id: string;
  name: string;
  kind: "receita" | "despesa";
  dre_group: string;
  system_key: string | null;
  active: boolean;
}

export type EntryKind = "receber" | "pagar";

export interface FinEntry {
  id: string;
  kind: EntryKind;
  description: string;
  category_id: string;
  category_name: string | null;
  customer_id: string | null;
  customer_name: string | null;
  counterparty: string | null;
  amount: Dec;
  issue_date: string;
  due_date: string;
  status: "aberto" | "pago" | "cancelado";
  paid_date: string | null;
  paid_amount: Dec | null;
  bank_account_id: string | null;
  bank_account_name: string | null;
  payment_method: string | null;
  document: string | null;
  installment: number | null;
  installments: number | null;
  group_id: string | null;
  sale_id: string | null;
  sale_number: number | null;
  notes: string | null;
}

export interface BankTx {
  id: string;
  bank_account_id: string;
  bank_account_name: string;
  date: string;
  amount: Dec;
  balance_after: Dec;
  description: string;
  category_id: string | null;
  category_name: string | null;
  entry_id: string | null;
  sale_id: string | null;
  transfer_group: string | null;
  reconciled: boolean;
  origin: "manual" | "titulo" | "venda" | "transferencia";
}

export interface OfxPreview {
  bank_id: string | null;
  account_id: string | null;
  start: string | null;
  end: string | null;
  ledger_balance: Dec | null;
  lines: { fitid: string; date: string; amount: Dec; memo: string; status: "novo" | "conciliar" | "ja_importado"; match_id: string | null; match_description: string | null }[];
}

export interface CashFlow {
  opening_balance: Dec;
  closing_balance: Dec;
  projected_closing: Dec;
  rows: { period: string; realized_in: Dec; realized_out: Dec; projected_in: Dec; projected_out: Dec; balance: Dec; projected_balance: Dec }[];
}

export interface Dre {
  months: string[];
  lines: { key: string; label: string; kind: "group" | "subtotal" | "info"; values: Dec[]; total: Dec }[];
}

export interface FinanceSummary {
  total_balance: Dec;
  receivable_overdue: Dec;
  receivable_overdue_count: number;
  receivable_next7: Dec;
  payable_overdue: Dec;
  payable_overdue_count: number;
  payable_next7: Dec;
  payable_today: Dec;
}

// ------------------------------------------------------------------ ordens de serviço

export type OsStatus = "aberta" | "em_andamento" | "aguardando" | "concluida" | "faturada" | "cancelada";

export interface OsItem {
  id: string;
  product_id: string;
  description: string;
  is_service: boolean;
  qty: Dec;
  unit_price: Dec;
  discount: Dec;
  total: Dec;
}

export interface OsEvent {
  id: string;
  kind: "criada" | "status" | "nota" | "faturada";
  from_status: OsStatus | null;
  to_status: OsStatus | null;
  note: string | null;
  user_name: string | null;
  created_at: string;
}

export interface ServiceOrder {
  id: string;
  number: number;
  customer_id: string;
  customer_name: string | null;
  customer_phone: string | null;
  status: OsStatus;
  equipment: string | null;
  reported: string;
  diagnosis: string | null;
  technician_id: string | null;
  technician_name: string | null;
  scheduled_date: string | null;
  due_date: string | null;
  subtotal: Dec;
  discount: Dec;
  total: Dec;
  sale_id: string | null;
  sale_number: number | null;
  finished_at: string | null;
  canceled_reason: string | null;
  created_at: string;
  updated_at: string;
  item_count: number;
  items: OsItem[] | null;
  events: OsEvent[] | null;
}

export interface OsItemInput {
  product_id: string;
  description?: string | null;
  qty: Dec;
  unit_price?: Dec | null;
  discount?: Dec | null;
}

export interface OsInput {
  customer_id: string;
  equipment?: string | null;
  reported: string;
  technician_id?: string | null;
  scheduled_date?: string | null;
  due_date?: string | null;
  items: OsItemInput[];
}
