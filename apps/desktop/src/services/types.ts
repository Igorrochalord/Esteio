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
  | "reports";

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
