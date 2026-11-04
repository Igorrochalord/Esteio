import { useMemo, useState, type FormEvent } from "react";

import { DataGrid, type GridColumn } from "../../components/DataGrid";
import { Dialog } from "../../components/Dialog";
import { Field } from "../../components/Field";
import { Notice } from "../../components/Notice";
import { useAsync } from "../../hooks/useAsync";
import { ApiError } from "../../services/api";
import { customersApi, financeApi } from "../../services/endpoints";
import { date, isoDay, money, parseDecimal, toNum } from "../../services/format";
import { PAYMENT_LABEL } from "../../services/labels";
import { entryStatus } from "../../services/finance";
import type { BankAccount, EntryKind, FinCategory, FinEntry } from "../../services/types";

const err = (e: unknown, fallback: string) => (e instanceof ApiError ? e.message : fallback);
const today = () => isoDay(new Date());

type Filter = "aberto" | "vencido" | "pago" | "cancelado" | "todos";

export function EntriesTab({ kind, accounts, categories, onChanged }: {
  kind: EntryKind;
  accounts: BankAccount[];
  categories: FinCategory[];
  onChanged: () => void;
}) {
  const [filter, setFilter] = useState<Filter>("aberto");
  const { data, error, loading, reload, setData } = useAsync(() => financeApi.entries({ kind }), [kind]);
  const [creating, setCreating] = useState(false);
  const [open, setOpen] = useState<FinEntry | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const isRec = kind === "receber";

  const rows = useMemo(
    () =>
      (data ?? []).filter((e) => {
        if (filter === "todos") return true;
        if (filter === "vencido") return e.status === "aberto" && e.due_date < today();
        return e.status === filter;
      }),
    [data, filter],
  );
  const overdue = (data ?? []).filter((e) => e.status === "aberto" && e.due_date < today());

  const columns: GridColumn<FinEntry>[] = [
    { key: "due", label: "Vencimento", kind: "date", get: (e) => `${e.due_date}T12:00:00` },
    { key: "description", label: "Descrição", get: (e) => e.description, width: 240 },
    { key: "who", label: isRec ? "Cliente" : "Fornecedor", get: (e) => e.customer_name ?? e.counterparty },
    { key: "category", label: "Categoria", get: (e) => e.category_name },
    { key: "amount", label: "Valor", kind: "money", get: (e) => e.amount, total: "sum" },
    {
      key: "status",
      label: "Situação",
      get: (e) => entryStatus(e).label,
      render: (e) => {
        const s = entryStatus(e);
        return <span className={`tag ${s.tone ? `tag--${s.tone}` : ""}`}>{s.label}</span>;
      },
    },
    { key: "paid", label: isRec ? "Recebido em" : "Pago em", kind: "date", get: (e) => (e.paid_date ? `${e.paid_date}T12:00:00` : null) },
    { key: "paid_amount", label: "Valor pago", kind: "money", get: (e) => e.paid_amount, total: "sum" },
    { key: "account", label: "Conta", get: (e) => e.bank_account_name },
    { key: "doc", label: "Documento", get: (e) => e.document },
  ];

  const replace = (e: FinEntry) => setData((list) => (list ?? []).map((x) => (x.id === e.id ? e : x)));

  return (
    <div className="stack">
      {error && <Notice tone="error">{error}</Notice>}
      {notice && <Notice tone="success">{notice}</Notice>}
      {overdue.length > 0 && filter !== "vencido" && (
        <Notice tone="error">
          {overdue.length} {isRec ? "conta(s) a receber vencida(s)" : "conta(s) a pagar vencida(s)"}: {money(overdue.reduce((a, e) => a + toNum(e.amount), 0))}.{" "}
          <button type="button" className="link-btn" onClick={() => setFilter("vencido")}>Ver vencidas</button>
        </Notice>
      )}
      {loading && !data ? (
        <p className="muted">Carregando…</p>
      ) : (
        <DataGrid
          rows={rows}
          columns={columns}
          rowKey={(e) => e.id}
          onRowOpen={setOpen}
          rowClass={(e) => (e.status === "cancelado" ? "is-canceled" : e.status === "aberto" && e.due_date < today() ? "is-warning" : undefined)}
          exportName={isRec ? "Contas a receber" : "Contas a pagar"}
          emptyText="Nenhum título aqui."
          initialSort={{ key: "due", dir: "asc" }}
          toolbar={
            <>
              <select aria-label="Situação" value={filter} onChange={(e) => setFilter(e.target.value as Filter)}>
                <option value="aberto">Em aberto</option>
                <option value="vencido">Vencidos</option>
                <option value="pago">{isRec ? "Recebidos" : "Pagos"}</option>
                <option value="cancelado">Cancelados</option>
                <option value="todos">Todos</option>
              </select>
              <button type="button" className="btn btn--primary" onClick={() => setCreating(true)}>
                {isRec ? "Nova conta a receber" : "Nova conta a pagar"}
              </button>
            </>
          }
        />
      )}
      <p className="muted small">Enter ou duplo clique num título para {isRec ? "receber" : "pagar"}, estornar ou cancelar.</p>

      <Dialog wide open={creating} title={isRec ? "Nova conta a receber" : "Nova conta a pagar"} onClose={() => setCreating(false)}>
        {creating && (
          <EntryForm
            kind={kind}
            categories={categories}
            onDone={async (created) => {
              setCreating(false);
              setNotice(created.length > 1 ? `${created.length} lançamentos criados.` : "Título lançado.");
              await reload();
              onChanged();
            }}
          />
        )}
      </Dialog>
      <Dialog open={!!open} title={open?.description ?? ""} onClose={() => setOpen(null)}>
        {open && (
          <EntryDetail
            entry={open}
            accounts={accounts}
            onChanged={(e, text) => {
              replace(e);
              setOpen(e);
              setNotice(text);
              onChanged();
            }}
          />
        )}
      </Dialog>
    </div>
  );
}

function EntryForm({ kind, categories, onDone }: { kind: EntryKind; categories: FinCategory[]; onDone: (e: FinEntry[]) => Promise<void> }) {
  const isRec = kind === "receber";
  const cats = categories.filter((c) => c.active && c.kind === (isRec ? "receita" : "despesa"));
  const { data: customers } = useAsync(() => (isRec ? customersApi.list() : Promise.resolve([])));
  const [form, setForm] = useState({
    description: "",
    category_id: cats[0]?.id ?? "",
    customer_id: "",
    counterparty: "",
    amount: "",
    due_date: today(),
    mode: "unico" as "unico" | "parcelado" | "recorrente",
    count: "2",
    document: "",
    notes: "",
  });
  const [error, setError] = useState<string | null>(null);
  const [fields, setFields] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const set = (k: keyof typeof form) => (e: { target: { value: string } }) => setForm({ ...form, [k]: e.target.value });
  const n = Math.max(1, Number(form.count) || 1);
  const total = toNum(parseDecimal(form.amount) ?? 0);

  async function submit(e: FormEvent) {
    e.preventDefault();
    const amount = parseDecimal(form.amount);
    if (!amount) return setFields({ amount: "Valor inválido" });
    setBusy(true);
    setError(null);
    setFields({});
    try {
      await onDone(
        await financeApi.createEntry({
          kind,
          description: form.description,
          category_id: form.category_id,
          customer_id: isRec && form.customer_id ? form.customer_id : null,
          counterparty: form.counterparty.trim() || null,
          amount,
          due_date: form.due_date,
          installments: form.mode === "parcelado" ? n : null,
          repeat_months: form.mode === "recorrente" ? n : null,
          document: form.document.trim() || null,
          notes: form.notes.trim() || null,
        }),
      );
    } catch (err2) {
      setError(err(err2, "Não foi possível lançar."));
      if (err2 instanceof ApiError) setFields(err2.fields);
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="stack" onSubmit={submit}>
      {error && <Notice tone="error">{error}</Notice>}
      <div className="form-grid">
        <Field label="Descrição" required autoFocus value={form.description} error={fields.description} onChange={set("description")}
          placeholder={isRec ? "Ex.: Serviço de manutenção" : "Ex.: Aluguel da loja"} />
        <div className="field">
          <label htmlFor="ef-cat">Categoria</label>
          <select id="ef-cat" value={form.category_id} onChange={set("category_id")}>
            {cats.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
          </select>
          {fields.category_id && <p className="field-error">{fields.category_id}</p>}
        </div>
        {isRec && (
          <div className="field">
            <label htmlFor="ef-cus">Cliente</label>
            <select id="ef-cus" value={form.customer_id} onChange={set("customer_id")}>
              <option value="">— sem cliente cadastrado —</option>
              {(customers ?? []).filter((c) => c.active).map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
            </select>
          </div>
        )}
        <Field label={isRec ? "Pagador (se não for cliente)" : "Fornecedor / favorecido"} value={form.counterparty} onChange={set("counterparty")} />
        <Field label={form.mode === "parcelado" ? "Valor total (R$)" : "Valor (R$)"} inputMode="decimal" required value={form.amount} error={fields.amount} onChange={set("amount")} />
        <Field label={form.mode === "unico" ? "Vencimento" : "1º vencimento"} type="date" required value={form.due_date} error={fields.due_date} onChange={set("due_date")} />
      </div>
      <div className="segmented" role="radiogroup" aria-label="Forma">
        {([["unico", "Único"], ["parcelado", "Parcelado"], ["recorrente", "Repetir todo mês"]] as const).map(([k, l]) => (
          <label key={k} className={form.mode === k ? "is-on" : undefined}>
            <input type="radio" name="mode" checked={form.mode === k} onChange={() => setForm({ ...form, mode: k })} />
            {l}
          </label>
        ))}
      </div>
      {form.mode !== "unico" && (
        <div className="row">
          <Field label={form.mode === "parcelado" ? "Número de parcelas" : "Quantos meses"} type="number" min={2} max={120} value={form.count} onChange={set("count")} />
          <p className="muted small">
            {form.mode === "parcelado"
              ? `${n}× de ${money(total / n)} (a diferença de centavos vai na última)`
              : `${n} lançamentos de ${money(total)}, um por mês`}
          </p>
        </div>
      )}
      <div className="form-grid">
        <Field label="Documento / nº da nota" value={form.document} onChange={set("document")} />
        <Field label="Observação" value={form.notes} onChange={set("notes")} />
      </div>
      <div className="row row--end">
        <button className="btn btn--primary" disabled={busy || !cats.length}>{busy ? "Salvando…" : "Lançar"}</button>
      </div>
    </form>
  );
}

function EntryDetail({ entry, accounts, onChanged }: {
  entry: FinEntry;
  accounts: BankAccount[];
  onChanged: (e: FinEntry, text: string) => void;
}) {
  const isRec = entry.kind === "receber";
  const active = accounts.filter((a) => a.active);
  const [form, setForm] = useState({
    date: today(),
    amount: entry.amount.replace(".", ","),
    account: active.find((a) => a.is_default)?.id ?? active[0]?.id ?? "",
    method: "pix",
  });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const st = entryStatus(entry);

  async function run(fn: () => Promise<FinEntry>, text: string) {
    setBusy(true);
    setError(null);
    try {
      onChanged(await fn(), text);
    } catch (e) {
      setError(err(e, "Não foi possível concluir."));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="stack">
      {error && <Notice tone="error">{error}</Notice>}
      <div className="kv-grid">
        <div><span>Valor</span><strong>{money(entry.amount)}</strong></div>
        <div><span>Vencimento</span><strong>{date(`${entry.due_date}T12:00:00`)}</strong></div>
        <div><span>Situação</span><strong><span className={`tag ${st.tone ? `tag--${st.tone}` : ""}`}>{st.label}</span></strong></div>
        <div><span>Categoria</span><strong>{entry.category_name}</strong></div>
        {(entry.customer_name || entry.counterparty) && <div><span>{isRec ? "Cliente" : "Fornecedor"}</span><strong>{entry.customer_name ?? entry.counterparty}</strong></div>}
        {entry.sale_number && <div><span>Venda</span><strong>nº {entry.sale_number}</strong></div>}
        {entry.paid_date && <div><span>{isRec ? "Recebido" : "Pago"} em</span><strong>{date(`${entry.paid_date}T12:00:00`)} · {money(entry.paid_amount)}</strong></div>}
        {entry.bank_account_name && <div><span>Conta</span><strong>{entry.bank_account_name}</strong></div>}
      </div>
      {entry.notes && <p className="muted">Obs.: {entry.notes}</p>}

      {entry.status === "aberto" && (
        <form
          className="stack panel panel--inset"
          onSubmit={(e) => {
            e.preventDefault();
            const amount = parseDecimal(form.amount);
            if (!amount) return setError("Valor inválido");
            void run(
              () => financeApi.pay(entry.id, { date: form.date, amount, bank_account_id: form.account || null, payment_method: form.method }),
              isRec ? "Recebimento registrado." : "Pagamento registrado.",
            );
          }}
        >
          <strong>{isRec ? "Registrar recebimento" : "Registrar pagamento"}</strong>
          <div className="form-grid">
            <Field label="Data" type="date" required value={form.date} onChange={(e) => setForm({ ...form, date: e.target.value })} />
            <Field label="Valor (com juros ou desconto)" inputMode="decimal" required value={form.amount} onChange={(e) => setForm({ ...form, amount: e.target.value })} />
            <div className="field">
              <label htmlFor="pay-acc">Conta</label>
              <select id="pay-acc" value={form.account} onChange={(e) => setForm({ ...form, account: e.target.value })}>
                {active.map((a) => <option key={a.id} value={a.id}>{a.name}</option>)}
              </select>
            </div>
            <div className="field">
              <label htmlFor="pay-met">Forma</label>
              <select id="pay-met" value={form.method} onChange={(e) => setForm({ ...form, method: e.target.value })}>
                {["pix", "dinheiro", "debito", "credito", "outro"].map((m) => <option key={m} value={m}>{PAYMENT_LABEL[m] ?? m}</option>)}
                <option value="boleto">Boleto</option>
                <option value="transferencia">Transferência</option>
              </select>
            </div>
          </div>
          <div className="row row--end">
            {!entry.sale_id && (
              <button type="button" className="btn btn--ghost" disabled={busy} onClick={() => void run(() => financeApi.cancel(entry.id), "Título cancelado.")}>
                Cancelar título
              </button>
            )}
            <button className="btn btn--primary" disabled={busy}>{isRec ? "Receber" : "Pagar"}</button>
          </div>
        </form>
      )}
      {entry.status === "pago" && (
        <div className="row row--end">
          <button type="button" className="btn btn--ghost" disabled={busy} onClick={() => void run(() => financeApi.unpay(entry.id), "Baixa estornada: o título voltou a ficar em aberto.")}>
            Estornar {isRec ? "recebimento" : "pagamento"}
          </button>
        </div>
      )}
    </div>
  );
}
