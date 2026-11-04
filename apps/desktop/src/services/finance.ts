// Regras de exibição do financeiro (compartilhadas entre telas).
import { isoDay } from "./format";
import type { FinEntry } from "./types";

const today = () => isoDay(new Date());

/** Situação exibida: vencido = em aberto com vencimento antes de hoje. */
export function entryStatus(e: FinEntry): { label: string; tone: "ok" | "warn" | "danger" | "" } {
  if (e.status === "pago") return { label: e.kind === "receber" ? "Recebido" : "Pago", tone: "ok" };
  if (e.status === "cancelado") return { label: "Cancelado", tone: "" };
  if (e.due_date < today()) return { label: "Vencido", tone: "danger" };
  if (e.due_date === today()) return { label: "Vence hoje", tone: "warn" };
  return { label: "Em aberto", tone: "" };
}
