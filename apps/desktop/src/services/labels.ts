import type { Role } from "./types";

export const ROLE_LABEL: Record<Role, string> = {
  owner: "Proprietário",
  admin: "Administrador",
  user: "Usuário",
};

export const ACTION_LABEL: Record<string, string> = {
  "company.create": "Empresa criada",
  "company.update": "Dados da empresa alterados",
  "branding.update": "Cores alteradas",
  "branding.logo_upload": "Logo enviado",
  "branding.logo_remove": "Logo removido",
  "user.create": "Pessoa adicionada",
  "user.update": "Pessoa alterada",
  "user.deactivate": "Pessoa desativada",
  "user.register": "Pediu cadastro",
  "user.approve": "Cadastro aprovado",
  "user.reject": "Cadastro recusado",
  "user.password_reset": "Senha redefinida",
  "auth.login": "Entrou no sistema",
  "backup.create": "Cópia de segurança criada",
  "backup.restore": "Cópia de segurança restaurada",
  "product.create": "Produto cadastrado",
  "product.update": "Produto alterado",
  "product.import": "Produtos importados de planilha",
  "category.create": "Categoria criada",
  "customer.create": "Cliente cadastrado",
  "customer.update": "Cliente alterado",
  "sale.create": "Venda registrada",
  "sale.cancel": "Venda cancelada",
  "bank_account.create": "Conta bancária criada",
  "bank_account.update": "Conta bancária alterada",
  "fin_category.create": "Categoria financeira criada",
  "fin_category.update": "Categoria financeira alterada",
  "fin_entry.create": "Título lançado",
  "fin_entry.update": "Título alterado",
  "fin_entry.receive": "Recebimento registrado",
  "fin_entry.pay": "Pagamento registrado",
  "fin_entry.unpay": "Baixa estornada",
  "fin_entry.cancel": "Título cancelado",
  "bank_tx.create": "Lançamento avulso",
  "bank_tx.transfer": "Transferência entre contas",
  "bank_tx.delete": "Lançamento excluído",
  "bank_tx.ofx_import": "Extrato importado",
  "os.create": "Ordem de serviço aberta",
  "os.invoice": "Ordem de serviço faturada",
  "os.cancel": "Ordem de serviço cancelada",
  "integration.create": "Integração configurada",
  "integration.update": "Integração alterada",
  "integration.delete": "Integração removida",
  "integration.job_retry": "Envio repetido",
  "integration.job_cancel": "Envio cancelado",
};

const dateTime = new Intl.DateTimeFormat("pt-BR", { dateStyle: "short", timeStyle: "short" });
export const formatDateTime = (iso: string | null) => (iso ? dateTime.format(new Date(iso)) : "—");

export function formatCnpj(digits: string | null): string {
  if (!digits || digits.length !== 14) return digits ?? "—";
  return digits.replace(/^(\d{2})(\d{3})(\d{3})(\d{4})(\d{2})$/, "$1.$2.$3/$4-$5");
}

/** Campos mostrados no histórico, com nome legível. Os demais (ids, chaves internas) ficam ocultos. */
export const FIELD_LABEL: Record<string, string> = {
  name: "nome",
  provider: "integração",
  environment: "ambiente",
  enabled: "ativada",
  credenciais: "credenciais",
  kind: "tipo",
  email: "e-mail",
  role: "papel",
  is_active: "ativo",
  cnpj: "CNPJ",
  primary_color: "cor principal",
  secondary_color: "cor do menu",
  file: "arquivo",
  code: "código",
  price: "preço",
  cost: "custo",
  unit: "unidade",
  min_stock: "estoque mínimo",
  track_stock: "controla estoque",
  active: "ativo",
  document: "CPF/CNPJ",
  phone: "telefone",
  city: "cidade",
  state: "UF",
  number: "nº",
  total: "total",
  payment_method: "pagamento",
  reason: "motivo",
  created: "criados",
  updated: "atualizados",
  safety_copy: "cópia do estado anterior",
};

export function formatValue(field: string, value: unknown): string {
  if (value === null || value === undefined || value === "") return "vazio";
  if (field === "role" && typeof value === "string" && value in ROLE_LABEL) {
    return ROLE_LABEL[value as Role].toLowerCase();
  }
  if (typeof value === "boolean") return value ? "sim" : "não";
  if (field === "cnpj") return formatCnpj(String(value));
  return String(value);
}

export const UNIT_OPTIONS = [
  { value: "un", label: "un" },
  { value: "kg", label: "kg" },
  { value: "g", label: "g" },
  { value: "l", label: "l" },
  { value: "ml", label: "ml" },
  { value: "cx", label: "cx" },
  { value: "pct", label: "pct" },
  { value: "m", label: "m" },
];

export const OS_STATUS_LABEL: Record<string, string> = {
  aberta: "Aberta",
  em_andamento: "Em andamento",
  aguardando: "Aguardando",
  concluida: "Concluída",
  faturada: "Faturada",
  cancelada: "Cancelada",
};

export const PAYMENT_LABEL: Record<string, string> = {
  dinheiro: "Dinheiro",
  pix: "Pix",
  credito: "Cartão de crédito",
  debito: "Cartão de débito",
  fiado: "Fiado",
  outro: "Outro",
};

export const MOVEMENT_LABEL: Record<string, string> = {
  entrada: "Entrada",
  saida: "Saída",
  ajuste: "Ajuste (contagem)",
  venda: "Venda",
  estorno_venda: "Cancelamento de venda",
};

export const UF_OPTIONS = [
  "AC", "AL", "AP", "AM", "BA", "CE", "DF", "ES", "GO", "MA", "MT", "MS", "MG", "PA",
  "PB", "PR", "PE", "PI", "RJ", "RN", "RS", "RO", "RR", "SC", "SP", "SE", "TO",
].map((uf) => ({ value: uf, label: uf }));

export function formatDocument(doc: string | null): string {
  if (!doc) return "";
  if (doc.length === 11) return doc.replace(/^(\d{3})(\d{3})(\d{3})(\d{2})$/, "$1.$2.$3-$4");
  return formatCnpj(doc);
}

export const ACCOUNT_KIND_LABEL: Record<string, string> = {
  caixa: "Caixa (dinheiro)",
  corrente: "Conta corrente",
  poupanca: "Poupança",
  carteira: "Carteira digital",
};

export const DRE_GROUP_LABEL: Record<string, string> = {
  receita_vendas: "Receita de vendas e serviços",
  outras_receitas: "Outras receitas",
  receitas_financeiras: "Receitas financeiras",
  deducoes: "Deduções",
  impostos: "Impostos",
  custos: "Custos (compras)",
  despesas_operacionais: "Despesas operacionais",
  despesas_pessoal: "Despesas com pessoal",
  despesas_financeiras: "Despesas financeiras",
  investimentos: "Investimentos",
  compras_estoque: "Compras de mercadoria (estoque)",
};
