// Formatação e leitura de números/datas no padrão brasileiro.
// Valores decimais trafegam como texto ("12.50"); Number só para exibir e somar na tela.

const brl = new Intl.NumberFormat("pt-BR", { style: "currency", currency: "BRL" });
const qtyFmt = new Intl.NumberFormat("pt-BR", { maximumFractionDigits: 3 });
const intFmt = new Intl.NumberFormat("pt-BR");
const pctFmt = new Intl.NumberFormat("pt-BR", { minimumFractionDigits: 1, maximumFractionDigits: 1 });
const dateFmt = new Intl.DateTimeFormat("pt-BR", { dateStyle: "short" });
const dateTimeFmt = new Intl.DateTimeFormat("pt-BR", { dateStyle: "short", timeStyle: "short" });

export const toNum = (v: string | number | null | undefined): number => (v == null || v === "" ? 0 : Number(v));

export const money = (v: string | number | null | undefined) => (v == null ? "—" : brl.format(toNum(v)));
export const qty = (v: string | number | null | undefined) => (v == null ? "—" : qtyFmt.format(toNum(v)));
export const int = (v: number | null | undefined) => (v == null ? "—" : intFmt.format(v));
export const pct = (v: string | number | null | undefined) => (v == null ? "—" : `${pctFmt.format(toNum(v))}%`);
export const date = (iso: string | null | undefined) => (iso ? dateFmt.format(new Date(iso)) : "—");
export const dateTime = (iso: string | null | undefined) => (iso ? dateTimeFmt.format(new Date(iso)) : "—");

/** Compacto para eixos: R$ 12,3 mil */
export const moneyShort = (v: number) =>
  new Intl.NumberFormat("pt-BR", { style: "currency", currency: "BRL", notation: "compact", maximumFractionDigits: 1 }).format(v);

/**
 * Lê o que a pessoa digitou ("1.234,56", "R$ 12,5", "0.8") e devolve texto decimal ("1234.56").
 * `null` se não for número.
 */
export function parseDecimal(raw: string): string | null {
  let s = raw.replace(/R\$/g, "").replace(/\s|\u00a0/g, "");
  if (!s) return null;
  const comma = s.lastIndexOf(",");
  const dot = s.lastIndexOf(".");
  if (comma >= 0 && dot >= 0) s = comma > dot ? s.replace(/\./g, "").replace(",", ".") : s.replace(/,/g, "");
  else if (comma >= 0) s = s.replace(",", ".");
  return /^-?\d+(\.\d+)?$/.test(s) ? s : null;
}

/** Decimal em texto → como a pessoa digita (vírgula). */
export const decimalInput = (v: string | null | undefined) => (v == null ? "" : v.replace(".", ","));

/** Data local (AAAA-MM-DD) → instante UTC da meia-noite local, para filtros. */
export function localDayStart(day: string): string {
  const [y, m, d] = day.split("-").map(Number);
  return new Date(y, m - 1, d).toISOString();
}

/** Dia seguinte (limite exclusivo do período). */
export function nextDay(day: string): string {
  const [y, m, d] = day.split("-").map(Number);
  return new Date(y, m - 1, d + 1).toISOString();
}

export function isoDay(dt: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${dt.getFullYear()}-${p(dt.getMonth() + 1)}-${p(dt.getDate())}`;
}

/** Minutos a somar ao UTC para chegar na hora local (Brasília = -180). */
export const tzOffset = () => -new Date().getTimezoneOffset();

/** Data/hora local sem fuso (para o Excel mostrar a hora certa). */
export function localIso(iso: string | null | undefined): string | null {
  if (!iso) return null;
  const dt = new Date(iso);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${isoDay(dt)}T${p(dt.getHours())}:${p(dt.getMinutes())}:${p(dt.getSeconds())}`;
}

/** Variação percentual entre atual e anterior (null se a base é zero). */
export const pctChange = (current: number, previous: number | null | undefined) =>
  previous == null || previous === 0 ? null : ((current - previous) / Math.abs(previous)) * 100;
