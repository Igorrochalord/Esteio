// Tema claro/escuro por computador (conveniência local; não vai para o banco).
export type ThemeMode = "system" | "light" | "dark";
const KEY = "nucleo.theme";

export function getThemeMode(): ThemeMode {
  try {
    const v = localStorage.getItem(KEY);
    return v === "light" || v === "dark" ? v : "system";
  } catch {
    return "system";
  }
}

export function applyThemeMode(mode: ThemeMode): void {
  const root = document.documentElement;
  if (mode === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", mode);
  try {
    if (mode === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, mode);
  } catch {
    /* sem armazenamento: vale só nesta sessão */
  }
}

/** Próximo modo no botão: automático → claro → escuro → automático. */
export const nextThemeMode = (m: ThemeMode): ThemeMode => (m === "system" ? "light" : m === "light" ? "dark" : "system");
