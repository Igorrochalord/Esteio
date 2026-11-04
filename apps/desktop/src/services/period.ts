// Períodos de filtro em dias locais (vendas e relatórios).
import { isoDay, localDayStart, nextDay } from "./format";

/** Período em dias locais, inclusivo nas duas pontas (AAAA-MM-DD). */
export interface Period {
  preset: PresetKey;
  from: string;
  to: string;
}

export type PresetKey = "today" | "yesterday" | "7d" | "30d" | "month" | "last_month" | "year" | "custom";

export const LABELS: Record<PresetKey, string> = {
  today: "Hoje",
  yesterday: "Ontem",
  "7d": "Últimos 7 dias",
  "30d": "Últimos 30 dias",
  month: "Este mês",
  last_month: "Mês passado",
  year: "Este ano",
  custom: "Personalizado",
};

export function presetPeriod(preset: PresetKey, base = new Date()): Period {
  const y = base.getFullYear();
  const m = base.getMonth();
  const d = base.getDate();
  const day = (yy: number, mm: number, dd: number) => isoDay(new Date(yy, mm, dd));
  switch (preset) {
    case "today":
      return { preset, from: day(y, m, d), to: day(y, m, d) };
    case "yesterday":
      return { preset, from: day(y, m, d - 1), to: day(y, m, d - 1) };
    case "7d":
      return { preset, from: day(y, m, d - 6), to: day(y, m, d) };
    case "30d":
      return { preset, from: day(y, m, d - 29), to: day(y, m, d) };
    case "last_month":
      return { preset, from: day(y, m - 1, 1), to: day(y, m, 0) };
    case "year":
      return { preset, from: day(y, 0, 1), to: day(y, m, d) };
    default:
      return { preset: preset === "custom" ? "custom" : "month", from: day(y, m, 1), to: day(y, m, d) };
  }
}

/** Limites para a API: início do primeiro dia e início do dia seguinte ao último (exclusivo). */
export const periodBounds = (p: Period) => ({ from: localDayStart(p.from), to: nextDay(p.to) });

/** Quantidade de dias do período (inclusivo). */
export function periodDays(p: Period): number {
  const a = new Date(localDayStart(p.from)).getTime();
  const b = new Date(nextDay(p.to)).getTime();
  return Math.max(1, Math.round((b - a) / 86_400_000));
}

export type CompareMode = "previous" | "last_year" | "none";

/** Período de comparação: o imediatamente anterior (mesmo tamanho) ou o mesmo intervalo um ano antes. */
export function comparePeriod(p: Period, mode: CompareMode): Period | null {
  if (mode === "none") return null;
  const parse = (d: string) => {
    const [y, m, dd] = d.split("-").map(Number);
    return new Date(y, m - 1, dd);
  };
  if (mode === "last_year") {
    const f = parse(p.from);
    const t = parse(p.to);
    f.setFullYear(f.getFullYear() - 1);
    t.setFullYear(t.getFullYear() - 1);
    return { preset: "custom", from: isoDay(f), to: isoDay(t) };
  }
  const days = periodDays(p);
  const to = parse(p.from);
  to.setDate(to.getDate() - 1);
  const from = new Date(to);
  from.setDate(from.getDate() - (days - 1));
  return { preset: "custom", from: isoDay(from), to: isoDay(to) };
}
