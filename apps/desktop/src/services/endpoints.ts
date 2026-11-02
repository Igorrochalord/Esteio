import { api } from "./api";
import type { AuditEntry, Branding, Me, Page, Role, Tenant, TokenPair, User } from "./types";

export const authApi = {
  login: (email: string, password: string) =>
    api<TokenPair>("/auth/login", { method: "POST", body: { email, password }, auth: false }),
  logout: (refresh_token: string) =>
    api<void>("/auth/logout", { method: "POST", body: { refresh_token }, auth: false }),
  me: () => api<Me>("/me"),
};

export interface OnboardingInput {
  company: { name: string; cnpj: string | null; primary_color: string; secondary_color: string };
  owner: { name: string; email: string; password: string };
}

export const tenantApi = {
  onboard: (data: OnboardingInput) =>
    api<TokenPair>("/onboarding", { method: "POST", body: data, auth: false }),
  get: () => api<Tenant>("/tenant"),
  update: (data: { name?: string; cnpj?: string | null }) =>
    api<Tenant>("/tenant", { method: "PUT", body: data }),
  updateBranding: (data: { primary_color?: string; secondary_color?: string }) =>
    api<Branding>("/tenant/branding", { method: "PUT", body: data }),
  uploadLogo: (file: File) => {
    const form = new FormData();
    form.append("file", file);
    return api<Branding>("/tenant/branding/logo", { method: "POST", form });
  },
  removeLogo: () => api<Branding>("/tenant/branding/logo", { method: "DELETE" }),
};

export const usersApi = {
  list: () => api<User[]>("/users"),
  create: (data: { name: string; email: string; password: string; role: Exclude<Role, "owner"> }) =>
    api<User>("/users", { method: "POST", body: data }),
  update: (id: string, data: { name?: string; role?: Exclude<Role, "owner">; is_active?: boolean }) =>
    api<User>(`/users/${id}`, { method: "PATCH", body: data }),
};

export const auditApi = {
  list: (params: { limit: number; offset: number; entity?: string }) => {
    const q = new URLSearchParams({ limit: String(params.limit), offset: String(params.offset) });
    if (params.entity) q.set("entity", params.entity);
    return api<Page<AuditEntry>>(`/audit?${q}`);
  },
};
