export type Role = "owner" | "admin" | "user";
export type Level = "read" | "write" | "manage";
export type Module = "settings" | "users" | "audit";

export interface TokenPair {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
}

export interface User {
  id: string;
  name: string;
  email: string;
  role: Role;
  is_active: boolean;
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
  tenant: Branding & { id: string; plan: string; subscription_status: string };
  permissions: Partial<Record<Module, Level>>;
}

export interface Tenant {
  id: string;
  name: string;
  cnpj: string | null;
  plan: string;
  subscription_status: string;
}

export interface AuditEntry {
  id: string;
  user_id: string | null;
  action: string;
  entity: string;
  entity_id: string | null;
  before: Record<string, unknown> | null;
  after: Record<string, unknown> | null;
  ip: string | null;
  created_at: string;
}

export interface Page<T> {
  items: T[];
  total: number;
  limit: number;
  offset: number;
}
