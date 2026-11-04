import { call } from "./api";
import type { AuditEntry, BackupInfo, Branding, Company, Me, Page, Role, User } from "./types";

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
