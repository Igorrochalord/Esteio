import { call } from "./api";
import type {
  AuditEntry,
  BackupInfo,
  Branding,
  Category,
  Company,
  Customer,
  CustomersReport,
  Dec,
  ImportField,
  ImportPreview,
  ImportResult,
  Me,
  Movement,
  Page,
  PaymentMethod,
  Product,
  ProductsReport,
  ReportQuery,
  Role,
  Sale,
  SalesSummary,
  SellerOption,
  SellerRow,
  SheetInput,
  StockReport,
  User,
} from "./types";

export const authApi = {
  setupStatus: () => call<{ configured: boolean }>("setup_status"),
  /** nome, cores e logo da empresa para a tela de entrada (sem login) */
  loginBranding: () => call<Branding | null>("login_branding"),
  /** pedido de acesso: fica aguardando aprovação de um administrador */
  register: (data: { name: string; email: string; password: string }) => call<void>("register", { data }),
  login: (email: string, password: string) => call<Me>("login", { email, password }),
  logout: () => call<void>("logout"),
  me: () => call<Me>("me"),
};

export interface OnboardingInput {
  company: { name: string; cnpj: string | null; primary_color: string; secondary_color: string };
  owner: { name: string; email: string; password: string };
}

/** Lê o arquivo como data URL (o núcleo confere se é PNG/JPEG de verdade). */
function readAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

export const companyApi = {
  onboard: (data: OnboardingInput) => call<Me>("onboard", { data }),
  get: () => call<Company>("company_get"),
  update: (data: { name?: string; cnpj?: string | null }) => call<Company>("company_update", { data }),
  updateBranding: (data: { primary_color?: string; secondary_color?: string }) =>
    call<Branding>("branding_update", { data }),
  uploadLogo: async (file: File) => call<Branding>("logo_upload", { content: await readAsDataUrl(file) }),
  removeLogo: () => call<Branding>("logo_remove"),
};

export const usersApi = {
  list: () => call<User[]>("users_list"),
  create: (data: { name: string; email: string; password: string; role: Exclude<Role, "owner"> }) =>
    call<User>("users_create", { data }),
  update: (id: string, data: { name?: string; role?: Exclude<Role, "owner">; is_active?: boolean }) =>
    call<User>("users_update", { user_id: id, data }),
  approve: (id: string, role: Exclude<Role, "owner">) => call<User>("users_approve", { user_id: id, role }),
  reject: (id: string) => call<void>("users_reject", { user_id: id }),
  resetPassword: (id: string, password: string) =>
    call<void>("users_reset_password", { user_id: id, password }),
};

export const auditApi = {
  list: (query: { limit: number; offset: number; entity?: string }) =>
    call<Page<AuditEntry>>("audit_list", { query }),
};

export const backupApi = {
  list: () => call<BackupInfo[]>("backup_list"),
  /** sem `dest`, salva na pasta de backups do app */
  create: (dest?: string) => call<BackupInfo>("backup_create", { dest: dest ?? null }),
  restore: (path: string) => call<BackupInfo>("backup_restore", { path }),
};

export interface ProductInput {
  code?: string;
  barcode?: string | null;
  name: string;
  category_id?: string | null;
  unit?: string;
  price: Dec;
  cost?: Dec | null;
  track_stock?: boolean;
  min_stock?: Dec | null;
  initial_stock?: Dec | null;
}

/** Edição parcial: só os campos enviados mudam; `null` limpa campos opcionais. */
export type ProductPatch = Partial<{
  code: string;
  barcode: string | null;
  name: string;
  category_id: string | null;
  unit: string;
  price: Dec;
  cost: Dec;
  track_stock: boolean;
  min_stock: Dec;
  active: boolean;
}>;

export const productsApi = {
  list: () => call<Product[]>("products_list"),
  create: (data: ProductInput) => call<Product>("products_create", { data }),
  update: (id: string, data: ProductPatch) => call<Product>("products_update", { id, data }),
  moveStock: (data: { product_id: string; kind: "entrada" | "saida" | "ajuste"; qty: Dec; unit_cost?: Dec | null; note?: string | null }) =>
    call<Product>("stock_move", { data }),
  movements: (query: { product_id?: string; from?: string; to?: string; limit?: number }) =>
    call<Movement[]>("stock_movements", { query }),
  categories: () => call<Category[]>("categories_list"),
  createCategory: (name: string) => call<Category>("categories_create", { name }),
  renameCategory: (id: string, name: string) => call<void>("categories_rename", { id, name }),
  deleteCategory: (id: string) => call<void>("categories_delete", { id }),
};

export type CustomerInput = Omit<Customer, "id" | "active" | "created_at" | "updated_at">;

export const customersApi = {
  list: () => call<Customer[]>("customers_list"),
  create: (data: Partial<CustomerInput> & { name: string }) => call<Customer>("customers_create", { data }),
  update: (id: string, data: Partial<CustomerInput & { active: boolean }>) =>
    call<Customer>("customers_update", { id, data }),
};

export interface SaleInput {
  customer_id?: string | null;
  seller_id?: string | null;
  payment_method: PaymentMethod;
  discount?: Dec | null;
  notes?: string | null;
  sold_at?: string | null;
  items: { product_id: string; qty: Dec; unit_price?: Dec | null; discount?: Dec | null }[];
}

export const salesApi = {
  list: (query: { from?: string; to?: string; status?: string; customer_id?: string; seller_id?: string; limit?: number }) =>
    call<Sale[]>("sales_list", { query }),
  get: (id: string) => call<Sale>("sales_get", { id }),
  create: (data: SaleInput) => call<Sale>("sales_create", { data }),
  cancel: (id: string, reason: string) => call<Sale>("sales_cancel", { id, reason }),
  sellers: () => call<SellerOption[]>("sellers_list"),
};

export const reportsApi = {
  sales: (query: ReportQuery) => call<SalesSummary>("report_sales", { query }),
  products: (query: ReportQuery) => call<ProductsReport>("report_products", { query }),
  stock: (query: ReportQuery) => call<StockReport>("report_stock", { query }),
  customers: (query: ReportQuery) => call<CustomersReport>("report_customers", { query }),
  sellers: (query: ReportQuery) => call<SellerRow[]>("report_sellers", { query }),
};

export const sheetApi = {
  exportXlsx: (path: string, sheets: SheetInput[]) => call<void>("export_xlsx", { path, sheets }),
  preview: (path: string) => call<ImportPreview>("import_preview", { path }),
  importProducts: (data: { path: string; mapping: Partial<Record<ImportField, number>>; update_existing: boolean }) =>
    call<ImportResult>("import_products", { data }),
};
