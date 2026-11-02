import type { Branding } from "./types";

export const DEFAULT_PRIMARY = "#2563eb";
export const DEFAULT_SECONDARY = "#0f172a";

function luminance(hex: string): number {
  const channel = (i: number) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

/** Texto legível sobre a cor: tinta escura em fundos claros, branco em escuros. */
export function inkFor(hex: string): string {
  return luminance(hex) > 0.42 ? "#16201d" : "#ffffff";
}

export function isHex(value: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(value);
}

/** Aplica as cores da empresa como variáveis CSS no documento inteiro. */
export function applyBranding(b: Pick<Branding, "primary_color" | "secondary_color"> | null): void {
  const primary = b && isHex(b.primary_color) ? b.primary_color : DEFAULT_PRIMARY;
  const secondary = b && isHex(b.secondary_color) ? b.secondary_color : DEFAULT_SECONDARY;
  const root = document.documentElement.style;
  root.setProperty("--brand", primary);
  root.setProperty("--brand-ink", inkFor(primary));
  root.setProperty("--brand-deep", secondary);
  root.setProperty("--brand-deep-ink", inkFor(secondary));
}

export function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter((w) => w.length > 2 || /^[A-ZÀ-Ú]/.test(w))
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase() ?? "")
    .join("");
}
