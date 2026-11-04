import { save } from "@tauri-apps/plugin-dialog";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from "react";

import { ApiError } from "../services/api";
import { sheetApi } from "../services/endpoints";
import { date, dateTime, decimalInput, isoDay, localIso, money, parseDecimal, pct, qty } from "../services/format";
import type { SheetColumn } from "../services/types";

/**
 * Grade tipo planilha: navegação por teclado, edição na célula, ordenação, busca,
 * totais e exportação para Excel do que está visível (já filtrado e ordenado).
 */

export type ColKind = "text" | "money" | "number" | "percent" | "date" | "datetime" | "select" | "boolean";
export type CellValue = string | number | boolean | null;

export interface GridColumn<T> {
  key: string;
  label: string;
  kind?: ColKind;
  /** valor bruto (decimais como texto "12.50") */
  get: (row: T) => CellValue;
  /** exibição personalizada (a busca e o Excel usam o valor formatado padrão) */
  render?: (row: T) => ReactNode;
  editable?: boolean | ((row: T) => boolean);
  /** opções de `select` (value "" = vazio) */
  options?: { value: string; label: string }[];
  width?: number;
  total?: "sum" | "count";
  /** campo pode ficar vazio ao editar (envia null) */
  nullable?: boolean;
}

interface Props<T> {
  rows: T[];
  columns: GridColumn<T>[];
  rowKey: (row: T) => string;
  /** grava uma célula; lance erro (ApiError) para recusar */
  onEdit?: (row: T, key: string, value: CellValue) => Promise<void>;
  /** Enter/duplo clique em célula não editável */
  onRowOpen?: (row: T) => void;
  rowClass?: (row: T) => string | undefined;
  /** filtros e botões ao lado da busca */
  toolbar?: ReactNode;
  /** nome da aba/arquivo na exportação; ausente = sem botão Exportar */
  exportName?: string;
  emptyText?: string;
  initialSort?: { key: string; dir: "asc" | "desc" };
  /** altura máxima da área com rolagem */
  maxHeight?: string;
}

const NUMERIC: ColKind[] = ["money", "number", "percent"];

function display<T>(col: GridColumn<T>, row: T): string {
  const v = col.get(row);
  if (v == null || v === "") return "";
  switch (col.kind) {
    case "money":
      return money(v as string);
    case "number":
      return qty(v as string);
    case "percent":
      return pct(v as string);
    case "date":
      return date(String(v));
    case "datetime":
      return dateTime(String(v));
    case "boolean":
      return v ? "Sim" : "Não";
    case "select":
      return col.options?.find((o) => o.value === v)?.label ?? String(v);
    default:
      return String(v);
  }
}

const fold = (s: string) =>
  s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "");

const collator = new Intl.Collator("pt-BR", { numeric: true, sensitivity: "base" });

function compare<T>(col: GridColumn<T>, a: T, b: T): number {
  const va = col.get(a);
  const vb = col.get(b);
  if (va == null || va === "") return vb == null || vb === "" ? 0 : 1; // vazios no fim
  if (vb == null || vb === "") return -1;
  if (NUMERIC.includes(col.kind ?? "text")) return Number(va) - Number(vb);
  if (col.kind === "boolean") return Number(va) - Number(vb);
  if (col.kind === "select") return collator.compare(display(col, a), display(col, b));
  return collator.compare(String(va), String(vb));
}

/** Soma exata em centavos/milésimos (evita 0,1 + 0,2 = 0,30000000004). */
function sum(values: CellValue[], scale: number): number {
  return values.reduce<number>((acc, v) => acc + Math.round(Number(v ?? 0) * scale), 0) / scale;
}

const isEditable = <T,>(col: GridColumn<T>, row: T) =>
  typeof col.editable === "function" ? col.editable(row) : !!col.editable;

export function DataGrid<T>(props: Props<T>) {
  const { rows, columns, rowKey, onEdit, onRowOpen, rowClass, toolbar, exportName, emptyText, maxHeight } = props;
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState(props.initialSort ?? null);
  const [active, setActive] = useState<{ r: number; c: number } | null>(null);
  const [editing, setEditingState] = useState<{ r: number; c: number; draft: string } | null>(null);
  // a sessão de edição é consumida uma única vez (Enter seguido do blur não grava duas vezes)
  const editRef = useRef<{ r: number; c: number; draft: string } | null>(null);
  const setEditing = (v: { r: number; c: number; draft: string } | null) => {
    editRef.current = v;
    setEditingState(v);
  };
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<{ tone: "error" | "info"; text: string } | null>(null);
  const tableRef = useRef<HTMLDivElement>(null);

  const visible = useMemo(() => {
    const q = fold(search.trim());
    let out = q
      ? rows.filter((row) => columns.some((col) => fold(display(col, row)).includes(q)))
      : rows.slice();
    if (sort) {
      const col = columns.find((c) => c.key === sort.key);
      if (col) {
        out = out.sort((a, b) => compare(col, a, b) * (sort.dir === "asc" ? 1 : -1));
      }
    }
    return out;
  }, [rows, columns, search, sort]);

  // mantém a célula ativa dentro dos limites quando a lista muda
  useEffect(() => {
    setActive((a) => (a && (a.r >= visible.length || a.c >= columns.length) ? null : a));
  }, [visible.length, columns.length]);

  const scrollTo = useCallback((r: number, c: number) => {
    tableRef.current
      ?.querySelector<HTMLElement>(`[data-cell="${r}:${c}"]`)
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, []);

  const move = (r: number, c: number) => {
    const nr = Math.max(0, Math.min(visible.length - 1, r));
    const nc = Math.max(0, Math.min(columns.length - 1, c));
    setActive({ r: nr, c: nc });
    requestAnimationFrame(() => scrollTo(nr, nc));
  };

  const startEdit = (r: number, c: number, draft?: string) => {
    const row = visible[r];
    const col = columns[c];
    if (!onEdit || !row || !isEditable(col, row)) {
      if (row && draft === undefined) onRowOpen?.(row);
      return;
    }
    if (col.kind === "boolean") {
      void commit(r, c, !col.get(row));
      return;
    }
    const raw = col.get(row);
    const initial =
      draft ??
      (NUMERIC.includes(col.kind ?? "text") ? decimalInput(raw as string | null) : raw == null ? "" : String(raw));
    setEditing({ r, c, draft: initial });
    setStatus(null);
  };

  async function commit(r: number, c: number, value: CellValue) {
    const row = visible[r];
    const col = columns[c];
    if (!onEdit || !row) return;
    setSaving(true);
    try {
      await onEdit(row, col.key, value);
      setStatus(null);
    } catch (e) {
      setStatus({ tone: "error", text: e instanceof ApiError ? e.message : "Não foi possível salvar." });
    } finally {
      setSaving(false);
    }
  }

  /** Converte o texto digitado para o valor da API; `undefined` = inválido. */
  function parseDraft(col: GridColumn<T>, draft: string): CellValue | undefined {
    const text = draft.trim();
    if (text === "") return col.nullable || col.kind === "select" ? null : col.kind === "text" ? "" : undefined;
    if (NUMERIC.includes(col.kind ?? "text")) return parseDecimal(text) ?? undefined;
    return text;
  }

  async function finishEdit(next: "down" | "right" | "left" | "stay") {
    const session = editRef.current;
    if (!session) return;
    const { r, c, draft } = session;
    const col = columns[c];
    const row = visible[r];
    const before = col.get(row);
    const value = parseDraft(col, draft);
    if (value === undefined) {
      setStatus({ tone: "error", text: `Valor inválido para "${col.label}".` });
      if (next === "stay") setEditing(null); // saiu do campo: descarta
      return;
    }
    setEditing(null);
    const same = value === before || (NUMERIC.includes(col.kind ?? "text") && value != null && before != null && Number(value) === Number(before));
    if (!same) await commit(r, c, value);
    if (next === "down") move(r + 1, c);
    else if (next === "right") move(r, c + 1);
    else if (next === "left") move(r, c - 1);
    tableRef.current?.focus();
  }

  function onKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    if (editing || !visible.length) return;
    const a = active ?? { r: 0, c: 0 };
    const col = columns[a.c];
    const row = visible[a.r];
    const ctrl = e.ctrlKey || e.metaKey;
    let handled = true;
    switch (e.key) {
      case "ArrowDown":
        move(a.r + 1, a.c);
        break;
      case "ArrowUp":
        move(a.r - 1, a.c);
        break;
      case "ArrowRight":
        move(a.r, a.c + 1);
        break;
      case "ArrowLeft":
        move(a.r, a.c - 1);
        break;
      case "Tab":
        move(a.r, a.c + (e.shiftKey ? -1 : 1));
        break;
      case "PageDown":
        move(a.r + 15, a.c);
        break;
      case "PageUp":
        move(a.r - 15, a.c);
        break;
      case "Home":
        move(ctrl ? 0 : a.r, 0);
        break;
      case "End":
        move(ctrl ? visible.length - 1 : a.r, columns.length - 1);
        break;
      case "Enter":
      case "F2":
        startEdit(a.r, a.c);
        break;
      default:
        handled = false;
    }
    if (!handled && active) {
      if (e.key === " " && col?.kind === "boolean") {
        handled = true;
        startEdit(a.r, a.c);
      } else if (e.key === "Delete" || e.key === "Backspace") {
        handled = true;
        startEdit(a.r, a.c, "");
      } else if (ctrl && e.key.toLowerCase() === "c") {
        handled = true;
        void navigator.clipboard
          ?.writeText(display(col, row))
          .then(() => setStatus({ tone: "info", text: "Copiado." }));
      } else if (
        e.key.length === 1 &&
        !ctrl &&
        !e.altKey &&
        col.kind !== "select" &&
        col.kind !== "boolean" &&
        isEditable(col, row)
      ) {
        handled = true;
        startEdit(a.r, a.c, e.key);
      }
    }
    if (handled) e.preventDefault();
  }

  function editorKeys(e: KeyboardEvent<HTMLInputElement | HTMLSelectElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      void finishEdit("down");
    } else if (e.key === "Tab") {
      e.preventDefault();
      void finishEdit(e.shiftKey ? "left" : "right");
    } else if (e.key === "Escape") {
      e.preventDefault();
      setEditing(null);
      tableRef.current?.focus();
    }
  }

  function toggleSort(key: string) {
    setSort((s) => (!s || s.key !== key ? { key, dir: "asc" } : s.dir === "asc" ? { key, dir: "desc" } : null));
  }

  async function exportXlsx() {
    if (!exportName) return;
    const path = await save({
      defaultPath: `${exportName}-${isoDay(new Date())}.xlsx`,
      filters: [{ name: "Planilha do Excel", extensions: ["xlsx"] }],
    });
    if (!path) return;
    const sheetCols: SheetColumn[] = columns.map((c) => ({
      label: c.label,
      kind: c.kind === "money" || c.kind === "number" || c.kind === "percent" || c.kind === "date" || c.kind === "datetime" ? c.kind : "text",
    }));
    const data = visible.map((row) =>
      columns.map((c) => {
        const v = c.get(row);
        if (c.kind === "date" || c.kind === "datetime") return localIso(v as string | null);
        if (c.kind === "select" || c.kind === "boolean") return display(c, row);
        return v;
      }),
    );
    try {
      await sheetApi.exportXlsx(path, [{ name: exportName, columns: sheetCols, rows: data }]);
      setStatus({ tone: "info", text: `Exportado: ${path}` });
    } catch (e) {
      setStatus({ tone: "error", text: e instanceof ApiError ? e.message : "Não foi possível exportar." });
    }
  }

  const hasTotals = columns.some((c) => c.total);
  const activeCol = active ? columns[active.c] : null;
  const activeRow = active ? visible[active.r] : null;

  return (
    <div className="grid">
      <div className="grid-toolbar">
        <input
          type="search"
          className="grid-search"
          placeholder="Buscar…"
          aria-label="Buscar na tabela"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        {toolbar}
        <span className="grid-spacer" />
        {exportName && (
          <button type="button" className="btn btn--ghost" onClick={() => void exportXlsx()} disabled={!visible.length}>
            Exportar Excel
          </button>
        )}
      </div>

      <div
        className="grid-scroll"
        style={maxHeight ? { maxHeight } : undefined}
        ref={tableRef}
        tabIndex={0}
        role="grid"
        aria-rowcount={visible.length}
        onKeyDown={onKeyDown}
      >
        <table className="grid-table">
          <thead>
            <tr>
              {columns.map((col) => {
                const dir = sort?.key === col.key ? sort.dir : null;
                return (
                  <th
                    key={col.key}
                    style={col.width ? { width: col.width, minWidth: col.width } : undefined}
                    className={NUMERIC.includes(col.kind ?? "text") ? "num" : undefined}
                    aria-sort={dir === "asc" ? "ascending" : dir === "desc" ? "descending" : "none"}
                  >
                    <button type="button" className="grid-sort" onClick={() => toggleSort(col.key)}>
                      {col.label}
                      <span className="grid-sort-icon" aria-hidden>
                        {dir === "asc" ? "▲" : dir === "desc" ? "▼" : ""}
                      </span>
                    </button>
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {visible.length === 0 && (
              <tr>
                <td className="grid-empty" colSpan={columns.length}>
                  {search ? "Nada encontrado para essa busca." : emptyText ?? "Nenhum registro."}
                </td>
              </tr>
            )}
            {visible.map((row, r) => (
              <tr key={rowKey(row)} className={rowClass?.(row)}>
                {columns.map((col, c) => {
                  const isActive = active?.r === r && active?.c === c;
                  const isEditing = editing?.r === r && editing?.c === c;
                  const editable = !!onEdit && isEditable(col, row);
                  const classes = [
                    NUMERIC.includes(col.kind ?? "text") ? "num" : "",
                    editable ? "is-editable" : "",
                    isActive ? "is-active" : "",
                    isEditing ? "is-editing" : "",
                  ]
                    .filter(Boolean)
                    .join(" ");
                  return (
                    <td
                      key={col.key}
                      className={classes}
                      data-cell={`${r}:${c}`}
                      role="gridcell"
                      aria-selected={isActive}
                      onMouseDown={() => setActive({ r, c })}
                      onDoubleClick={() => startEdit(r, c)}
                    >
                      {isEditing && editing ? (
                        col.kind === "select" ? (
                          <select
                            autoFocus
                            className="grid-editor"
                            value={editing.draft}
                            onChange={(e) => setEditing({ ...editing, draft: e.target.value })}
                            onBlur={() => void finishEdit("stay")}
                            onKeyDown={editorKeys}
                          >
                            {col.nullable && <option value="">—</option>}
                            {col.options?.map((o) => (
                              <option key={o.value} value={o.value}>
                                {o.label}
                              </option>
                            ))}
                          </select>
                        ) : (
                          <input
                            autoFocus
                            className="grid-editor"
                            inputMode={NUMERIC.includes(col.kind ?? "text") ? "decimal" : undefined}
                            value={editing.draft}
                            onChange={(e) => setEditing({ ...editing, draft: e.target.value })}
                            onBlur={() => void finishEdit("stay")}
                            onKeyDown={editorKeys}
                          />
                        )
                      ) : col.kind === "boolean" ? (
                        <input
                          type="checkbox"
                          tabIndex={-1}
                          checked={!!col.get(row)}
                          disabled={!editable || saving}
                          aria-label={col.label}
                          onChange={() => void commit(r, c, !col.get(row))}
                        />
                      ) : col.render ? (
                        col.render(row)
                      ) : (
                        display(col, row) || <span className="grid-blank">—</span>
                      )}
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
          {hasTotals && visible.length > 0 && (
            <tfoot>
              <tr>
                {columns.map((col, i) => {
                  let text = i === 0 ? "Total" : "";
                  if (col.total === "count") text = `${visible.length}`;
                  if (col.total === "sum") {
                    const scale = col.kind === "number" ? 1000 : 100;
                    const s = sum(visible.map((row) => col.get(row)), scale);
                    text = col.kind === "money" ? money(s) : qty(s);
                  }
                  return (
                    <td key={col.key} className={NUMERIC.includes(col.kind ?? "text") ? "num" : undefined}>
                      {text}
                    </td>
                  );
                })}
              </tr>
            </tfoot>
          )}
        </table>
      </div>

      <div className="grid-status" aria-live="polite">
        <span>
          {visible.length === rows.length ? `${visible.length} linha(s)` : `${visible.length} de ${rows.length} linha(s)`}
        </span>
        {activeCol && activeRow && (
          <span className="muted">
            {activeCol.label}: {display(activeCol, activeRow) || "—"}
            {onEdit && isEditable(activeCol, activeRow) ? " · Enter ou digite para editar" : ""}
          </span>
        )}
        {saving && <span className="muted">Salvando…</span>}
        {status && <span className={status.tone === "error" ? "grid-status-error" : "muted"}>{status.text}</span>}
      </div>
    </div>
  );
}
