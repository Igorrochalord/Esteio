export type Role = "owner" | "admin" | "user";
export type Level = "read" | "write" | "manage";
export type Module = "settings" | "users" | "audit" | "backup";

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
