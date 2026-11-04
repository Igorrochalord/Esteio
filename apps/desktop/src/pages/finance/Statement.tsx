import { open as openFile } from "@tauri-apps/plugin-dialog";
import { useState, type FormEvent } from "react";

import { DataGrid, type GridColumn } from "../../components/DataGrid";
import { Dialog } from "../../components/Dialog";
import { Field } from "../../components/Field";
import { Notice } from "../../components/Notice";
import { PeriodPicker } from "../../components/PeriodPicker";
import { useAsync } from "../../hooks/useAsync";
import { ApiError } from "../../services/api";
import { financeApi } from "../../services/endpoints";
import { date, isoDay, money, parseDecimal } from "../../services/format";
import { presetPeriod, type Period } from "../../services/period";
import type { BankAccount, BankTx, FinCategory, OfxPreview } from "../../services/types";

const err = (e: unknown, fallback: string) => (e instanceof ApiError ? e.message : fallback);
const ORIGIN: Record<BankTx["origin"], string> = { manual: "Avulso", titulo: "Título", venda: "Venda", transferencia: "Transferência" };

export function StatementTab({ accounts, categories, onChanged }: { accounts: BankAccount[]; categories: FinCategory[]; onChanged: () => void }) {
  const [account, setAccount] = useState("");
  const [period, setPeriod] = useState<Period>(() => presetPeriod("30d"));
  const { data, error, loading, reload } = useAsync(
    () => financeApi.transactions({ bank_account_id: account || undefined, from: period.from, to: period.to }),
    [account, period.from, period.to],
  );
  const [dialog, setDialog] = useState<"tx" | "transfer" | "ofx" | null>(null);
  const [selected, setSelected] = useState<BankTx | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const done = async (text: string) => {
    setDialog(null);
    setSelected(null);
    setNotice(text);
    await reload();
    onChanged();
  };

  const columns: GridColumn<BankTx>[] = [
    { key: "date", label: "Data", kind: "date", get: (t) => `${t.date}T12:00:00` },
    ...(account ? [] : ([{ key: "account", label: "Conta", get: (t) => t.bank_account_name }] satisfies GridColumn<BankTx>[])),
    { key: "description", label: "Descrição", get: (t) => t.description, width: 260 },
    { key: "category", label: "Categoria", get: (t) => t.category_name },
    { key: "origin", label: "Origem", get: (t) => ORIGIN[t.origin] },
    {
      key: "amount",
      label: "Valor",
      kind: "money",
      get: (t) => t.amount,
      render: (t) => <span className={t.amount.startsWith("-") ? "neg" : "pos"}>{money(t.amount)}</span>,
      total: "sum",
    },
    ...(account ? ([{ key: "balance", label: "Saldo", kind: "money", get: (t) => t.balance_after }] satisfies GridColumn<BankTx>[]) : []),
    { key: "reconciled", label: "Conciliado", kind: "boolean", get: (t) => t.reconciled },
  ];

  return (
    <div className="stack">
      {(error || actionError) && <Notice tone="error">{error ?? actionError}</Notice>}
      {notice && <Notice tone="success">{notice}</Notice>}
      {loading && !data ? (
        <p className="muted">Carregando…</p>
      ) : (
        <DataGrid
          rows={data ?? []}
          columns={columns}
          rowKey={(t) => t.id}
          onRowOpen={setSelected}
          exportName="Extrato"
          emptyText="Nenhum lançamento no período."
          initialSort={{ key: "date", dir: "desc" }}
          toolbar={
            <>
              <select aria-label="Conta" value={account} onChange={(e) => setAccount(e.target.value)}>
                <option value="">Todas as contas</option>
                {accounts.map((a) => <option key={a.id} value={a.id}>{a.name}</option>)}
              </select>
              <PeriodPicker value={period} onChange={setPeriod} />
              <button type="button" className="btn btn--ghost" onClick={() => setDialog("ofx")}>Importar extrato (OFX)</button>
              <button type="button" className="btn btn--ghost" onClick={() => setDialog("transfer")}>Transferência</button>
              <button type="button" className="btn btn--primary" onClick={() => setDialog("tx")}>Lançamento avulso</button>
            </>
          }
        />
      )}
      <p className="muted small">Selecione uma conta para ver o saldo após cada lançamento. Enter num lançamento avulso ou transferência permite excluir.</p>

      <Dialog open={dialog === "tx"} title="Lançamento avulso" onClose={() => setDialog(null)}>
        {dialog === "tx" && <TxForm accounts={accounts} categories={categories} defaultAccount={account} onDone={() => done("Lançamento registrado.")} />}
      </Dialog>
      <Dialog open={dialog === "transfer"} title="Transferência entre contas" onClose={() => setDialog(null)}>
        {dialog === "transfer" && <TransferForm accounts={accounts} onDone={() => done("Transferência registrada.")} />}
      </Dialog>
      <Dialog wide open={dialog === "ofx"} title="Importar extrato bancário (OFX)" onClose={() => setDialog(null)}>
        {dialog === "ofx" && <OfxWizard accounts={accounts} categories={categories} defaultAccount={account} onDone={(t) => done(t)} />}
      </Dialog>
      <Dialog open={!!selected} title="Lançamento" onClose={() => setSelected(null)}>
        {selected && (
          <div className="stack">
            <div className="kv-grid">
              <div><span>Data</span><strong>{date(`${selected.date}T12:00:00`)}</strong></div>
              <div><span>Valor</span><strong>{money(selected.amount)}</strong></div>
              <div><span>Conta</span><strong>{selected.bank_account_name}</strong></div>
              <div><span>Origem</span><strong>{ORIGIN[selected.origin]}</strong></div>
            </div>
            <p>{selected.description}</p>
            {selected.origin === "manual" || selected.origin === "transferencia" ? (
              <div className="row row--end">
                <button
                  type="button"
                  className="btn btn--ghost"
                  onClick={async () => {
                    try {
                      await financeApi.deleteTransaction(selected.id);
                      await done(selected.origin === "transferencia" ? "Transferência excluída (as duas pontas)." : "Lançamento excluído.");
                    } catch (e) {
                      setSelected(null);
                      setNotice(null);
                      setActionError(err(e, "Não foi possível excluir."));
                    }
                  }}
                >
                  Excluir lançamento
                </button>
              </div>
            ) : (
              <p className="muted small">
                {selected.origin === "venda" ? "Lançamento de venda: para desfazer, cancele a venda." : "Baixa de título: para desfazer, estorne no título."}
              </p>
            )}
          </div>
        )}
      </Dialog>
    </div>
  );
}

function TxForm({ accounts, categories, defaultAccount, onDone }: { accounts: BankAccount[]; categories: FinCategory[]; defaultAccount: string; onDone: () => void }) {
  const active = accounts.filter((a) => a.active);
  const [form, setForm] = useState({ account: defaultAccount || active[0]?.id || "", kind: "saida" as "entrada" | "saida", amount: "", description: "", date: isoDay(new Date()), category: "" });
  const [error, setError] = useState<string | null>(null);
  const cats = categories.filter((c) => c.active && c.kind === (form.kind === "entrada" ? "receita" : "despesa"));
  async function submit(e: FormEvent) {
    e.preventDefault();
    const amount = parseDecimal(form.amount);
    if (!amount) return setError("Valor inválido");
    try {
      await financeApi.createTransaction({ bank_account_id: form.account, date: form.date, kind: form.kind, amount, description: form.description, category_id: form.category || null });
      onDone();
    } catch (e2) {
      setError(err(e2, "Não foi possível lançar."));
    }
  }
  return (
    <form className="stack" onSubmit={submit}>
      {error && <Notice tone="error">{error}</Notice>}
      <div className="segmented" role="radiogroup" aria-label="Tipo">
        {(["entrada", "saida"] as const).map((k) => (
          <label key={k} className={form.kind === k ? "is-on" : undefined}>
            <input type="radio" checked={form.kind === k} onChange={() => setForm({ ...form, kind: k, category: "" })} />
            {k === "entrada" ? "Entrada" : "Saída"}
          </label>
        ))}
      </div>
      <div className="form-grid">
        <div className="field">
          <label htmlFor="tx-acc">Conta</label>
          <select id="tx-acc" value={form.account} onChange={(e) => setForm({ ...form, account: e.target.value })}>
            {active.map((a) => <option key={a.id} value={a.id}>{a.name}</option>)}
          </select>
        </div>
        <Field label="Data" type="date" required value={form.date} onChange={(e) => setForm({ ...form, date: e.target.value })} />
        <Field label="Valor (R$)" inputMode="decimal" required autoFocus value={form.amount} onChange={(e) => setForm({ ...form, amount: e.target.value })} />
        <div className="field">
          <label htmlFor="tx-cat">Categoria</label>
          <select id="tx-cat" value={form.category} onChange={(e) => setForm({ ...form, category: e.target.value })}>
            <option value="">— sem categoria (não entra na DRE) —</option>
            {cats.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}
          </select>
        </div>
      </div>
      <Field label="Descrição" required value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} placeholder="Ex.: Tarifa bancária" />
      <div className="row row--end"><button className="btn btn--primary">Lançar</button></div>
    </form>
  );
}

function TransferForm({ accounts, onDone }: { accounts: BankAccount[]; onDone: () => void }) {
  const active = accounts.filter((a) => a.active);
  const [form, setForm] = useState({ from: active[0]?.id ?? "", to: active[1]?.id ?? "", amount: "", date: isoDay(new Date()), description: "" });
  const [error, setError] = useState<string | null>(null);
  async function submit(e: FormEvent) {
    e.preventDefault();
    const amount = parseDecimal(form.amount);
    if (!amount) return setError("Valor inválido");
    try {
      await financeApi.transfer({ from_account_id: form.from, to_account_id: form.to, amount, date: form.date, description: form.description || null });
      onDone();
    } catch (e2) {
      setError(err(e2, "Não foi possível transferir."));
    }
  }
  if (active.length < 2) return <p className="muted">Cadastre pelo menos duas contas ativas para transferir.</p>;
  return (
    <form className="stack" onSubmit={submit}>
      {error && <Notice tone="error">{error}</Notice>}
      <div className="form-grid">
        <div className="field">
          <label htmlFor="tr-from">De</label>
          <select id="tr-from" value={form.from} onChange={(e) => setForm({ ...form, from: e.target.value })}>
            {active.map((a) => <option key={a.id} value={a.id}>{a.name} ({money(a.balance)})</option>)}
          </select>
        </div>
        <div className="field">
          <label htmlFor="tr-to">Para</label>
          <select id="tr-to" value={form.to} onChange={(e) => setForm({ ...form, to: e.target.value })}>
            {active.map((a) => <option key={a.id} value={a.id}>{a.name}</option>)}
          </select>
        </div>
        <Field label="Valor (R$)" inputMode="decimal" required autoFocus value={form.amount} onChange={(e) => setForm({ ...form, amount: e.target.value })} />
        <Field label="Data" type="date" required value={form.date} onChange={(e) => setForm({ ...form, date: e.target.value })} />
      </div>
      <Field label="Descrição" value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} placeholder="Ex.: Depósito do caixa no banco" />
      <div className="row row--end"><button className="btn btn--primary">Transferir</button></div>
    </form>
  );
}

const OFX_STATUS: Record<string, { label: string; tone: string }> = {
  conciliar: { label: "Conciliar", tone: "tag--ok" },
  novo: { label: "Novo", tone: "" },
  ja_importado: { label: "Já importado", tone: "" },
};

function OfxWizard({ accounts, categories, defaultAccount, onDone }: { accounts: BankAccount[]; categories: FinCategory[]; defaultAccount: string; onDone: (text: string) => void }) {
  const active = accounts.filter((a) => a.active);
  const [account, setAccount] = useState(defaultAccount || active.find((a) => a.kind !== "caixa")?.id || active[0]?.id || "");
  const [path, setPath] = useState<string | null>(null);
  const [preview, setPreview] = useState<OfxPreview | null>(null);
  const [createNew, setCreateNew] = useState(true);
  const income = categories.filter((c) => c.kind === "receita" && c.active);
  const expense = categories.filter((c) => c.kind === "despesa" && c.active);
  const [incomeCat, setIncomeCat] = useState(income.find((c) => c.name === "Outras receitas")?.id ?? income[0]?.id ?? "");
  const [expenseCat, setExpenseCat] = useState(expense.find((c) => c.name === "Outras despesas")?.id ?? expense[0]?.id ?? "");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function choose() {
    const file = await openFile({ multiple: false, directory: false, filters: [{ name: "Extrato OFX", extensions: ["ofx", "OFX"] }] });
    if (typeof file !== "string") return;
    setPath(file);
    await load(file, account);
  }
  async function load(file: string, acc: string) {
    setError(null);
    try {
      setPreview(await financeApi.ofxPreview(acc, file));
    } catch (e) {
      setPreview(null);
      setError(err(e, "Não foi possível ler o extrato."));
    }
  }
  async function run() {
    if (!path) return;
    setBusy(true);
    try {
      const r = await financeApi.ofxImport({ bank_account_id: account, path, create_new: createNew, income_category_id: incomeCat || null, expense_category_id: expenseCat || null });
      onDone(`Extrato importado: ${r.reconciled} conciliado(s), ${r.created} novo(s), ${r.skipped} ignorado(s).`);
    } catch (e) {
      setError(err(e, "Não foi possível importar."));
    } finally {
      setBusy(false);
    }
  }
  const counts = (s: string) => preview?.lines.filter((l) => l.status === s).length ?? 0;
  return (
    <div className="stack">
      {error && <Notice tone="error">{error}</Notice>}
      <p className="muted">
        Baixe o extrato no internet banking no formato <strong>OFX</strong> (às vezes chamado “Money” ou “Quicken”). O sistema concilia com o
        que já foi lançado (mesmo valor, até 3 dias de diferença) e pode criar o que faltar. Importar o mesmo arquivo de novo não duplica nada.
      </p>
      <div className="row">
        <select aria-label="Conta do extrato" value={account} onChange={(e) => { setAccount(e.target.value); if (path) void load(path, e.target.value); }}>
          {active.map((a) => <option key={a.id} value={a.id}>{a.name}</option>)}
        </select>
        <button type="button" className="btn btn--ghost" onClick={() => void choose()}>{path ? "Escolher outro arquivo" : "Escolher arquivo OFX…"}</button>
      </div>
      {preview && (
        <>
          <p>
            <span className="tag tag--ok">{counts("conciliar")} a conciliar</span>{" "}
            <span className="tag">{counts("novo")} novo(s)</span>{" "}
            <span className="tag">{counts("ja_importado")} já importado(s)</span>
            {preview.ledger_balance && <span className="muted small"> · saldo no banco: {money(preview.ledger_balance)}</span>}
          </p>
          <div className="table-wrap" style={{ maxHeight: "40vh", overflow: "auto" }}>
            <table className="table small">
              <thead><tr><th>Data</th><th>Descrição no banco</th><th className="num">Valor</th><th>Situação</th><th>Corresponde a</th></tr></thead>
              <tbody>
                {preview.lines.map((l) => (
                  <tr key={l.fitid}>
                    <td className="nowrap">{date(`${l.date}T12:00:00`)}</td>
                    <td>{l.memo}</td>
                    <td className={`num ${l.amount.startsWith("-") ? "neg" : "pos"}`}>{money(l.amount)}</td>
                    <td><span className={`tag ${OFX_STATUS[l.status].tone}`}>{OFX_STATUS[l.status].label}</span></td>
                    <td className="muted">{l.match_description ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <label className="check">
            <input type="checkbox" checked={createNew} onChange={(e) => setCreateNew(e.target.checked)} />
            Criar lançamentos para as linhas novas
          </label>
          {createNew && (
            <div className="form-grid">
              <div className="field">
                <label htmlFor="ofx-in">Categoria das entradas novas</label>
                <select id="ofx-in" value={incomeCat} onChange={(e) => setIncomeCat(e.target.value)}>{income.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</select>
              </div>
              <div className="field">
                <label htmlFor="ofx-out">Categoria das saídas novas</label>
                <select id="ofx-out" value={expenseCat} onChange={(e) => setExpenseCat(e.target.value)}>{expense.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</select>
              </div>
            </div>
          )}
          <div className="row row--end">
            <button type="button" className="btn btn--primary" disabled={busy} onClick={() => void run()}>{busy ? "Importando…" : "Importar e conciliar"}</button>
          </div>
        </>
      )}
    </div>
  );
}
