import { useState, type FormEvent } from "react";

import { DataGrid, type CellValue, type GridColumn } from "../../components/DataGrid";
import { Dialog } from "../../components/Dialog";
import { Field } from "../../components/Field";
import { Notice } from "../../components/Notice";
import { ApiError } from "../../services/api";
import { financeApi } from "../../services/endpoints";
import { isoDay, parseDecimal } from "../../services/format";
import { ACCOUNT_KIND_LABEL, DRE_GROUP_LABEL } from "../../services/labels";
import type { BankAccount, FinCategory } from "../../services/types";

const err = (e: unknown, fallback: string) => (e instanceof ApiError ? e.message : fallback);
const KIND_OPTIONS = Object.entries(ACCOUNT_KIND_LABEL).map(([value, label]) => ({ value, label }));
const DRE_OPTIONS = Object.entries(DRE_GROUP_LABEL).map(([value, label]) => ({ value, label }));

export function AccountsTab({ accounts, onChanged }: { accounts: BankAccount[]; onChanged: () => Promise<void> }) {
  const [creating, setCreating] = useState(false);
  const columns: GridColumn<BankAccount>[] = [
    { key: "name", label: "Nome", get: (a) => a.name, editable: true, width: 200 },
    { key: "kind", label: "Tipo", kind: "select", get: (a) => a.kind, options: KIND_OPTIONS, editable: true },
    { key: "bank_name", label: "Banco", get: (a) => a.bank_name, editable: true, nullable: true },
    { key: "agency", label: "Agência", get: (a) => a.agency, editable: true, nullable: true },
    { key: "account_number", label: "Conta", get: (a) => a.account_number, editable: true, nullable: true },
    { key: "initial_balance", label: "Saldo inicial", kind: "money", get: (a) => a.initial_balance, editable: true },
    { key: "balance", label: "Saldo atual", kind: "money", get: (a) => a.balance, total: "sum" },
    { key: "unreconciled", label: "Não conciliados", kind: "number", get: (a) => a.unreconciled },
    { key: "is_default", label: "Recebe vendas", kind: "boolean", get: (a) => a.is_default, editable: (a) => !a.is_default },
    { key: "active", label: "Ativa", kind: "boolean", get: (a) => a.active, editable: true },
  ];
  async function onEdit(a: BankAccount, key: string, value: CellValue) {
    await financeApi.updateAccount(a.id, { [key]: key === "initial_balance" ? String(value ?? "0") : value });
    await onChanged();
  }
  return (
    <div className="stack">
      <DataGrid
        rows={accounts}
        columns={columns}
        rowKey={(a) => a.id}
        onEdit={onEdit}
        rowClass={(a) => (a.active ? undefined : "is-muted")}
        exportName="Contas bancárias"
        emptyText="Nenhuma conta ainda. A primeira venda à vista cria a conta Caixa sozinha."
        toolbar={<button type="button" className="btn btn--primary" onClick={() => setCreating(true)}>Nova conta</button>}
      />
      <p className="muted small">
        “Recebe vendas” é a conta onde entram as vendas à vista. {/* PLANO: Gratuito até 3 contas */}No plano Gratuito (futuro), até 3 contas;
        hoje está liberado.
      </p>
      <Dialog open={creating} title="Nova conta" onClose={() => setCreating(false)}>
        {creating && (
          <AccountForm
            onDone={async () => {
              setCreating(false);
              await onChanged();
            }}
          />
        )}
      </Dialog>
    </div>
  );
}

function AccountForm({ onDone }: { onDone: () => Promise<void> }) {
  const [form, setForm] = useState({ name: "", kind: "corrente", bank_name: "", agency: "", account_number: "", initial: "0", opening: isoDay(new Date()) });
  const [error, setError] = useState<string | null>(null);
  const set = (k: keyof typeof form) => (e: { target: { value: string } }) => setForm({ ...form, [k]: e.target.value });
  async function submit(e: FormEvent) {
    e.preventDefault();
    const initial = parseDecimal(form.initial || "0");
    if (initial == null) return setError("Saldo inicial inválido");
    try {
      await financeApi.createAccount({
        name: form.name,
        kind: form.kind,
        bank_name: form.bank_name || null,
        agency: form.agency || null,
        account_number: form.account_number || null,
        initial_balance: initial,
        opening_date: form.opening,
      });
      await onDone();
    } catch (e2) {
      setError(err(e2, "Não foi possível criar a conta."));
    }
  }
  return (
    <form className="stack" onSubmit={submit}>
      {error && <Notice tone="error">{error}</Notice>}
      <div className="form-grid">
        <Field label="Nome" required autoFocus value={form.name} onChange={set("name")} placeholder="Ex.: Itaú PJ" />
        <div className="field">
          <label htmlFor="ac-kind">Tipo</label>
          <select id="ac-kind" value={form.kind} onChange={set("kind")}>{KIND_OPTIONS.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}</select>
        </div>
        <Field label="Banco" value={form.bank_name} onChange={set("bank_name")} />
        <Field label="Agência" value={form.agency} onChange={set("agency")} />
        <Field label="Conta" value={form.account_number} onChange={set("account_number")} />
        <Field label="Saldo inicial (R$)" inputMode="decimal" value={form.initial} onChange={set("initial")} />
        <Field label="Data do saldo inicial" type="date" value={form.opening} onChange={set("opening")} />
      </div>
      <div className="row row--end"><button className="btn btn--primary">Criar conta</button></div>
    </form>
  );
}

export function CategoriesTab({ categories, onChanged }: { categories: FinCategory[]; onChanged: () => Promise<void> }) {
  const [form, setForm] = useState({ name: "", kind: "despesa", dre_group: "despesas_operacionais" });
  const [error, setError] = useState<string | null>(null);
  const columns: GridColumn<FinCategory>[] = [
    { key: "name", label: "Nome", get: (c) => c.name, editable: (c) => !c.system_key, width: 240 },
    { key: "kind", label: "Tipo", get: (c) => (c.kind === "receita" ? "Receita" : "Despesa") },
    { key: "dre_group", label: "Linha da DRE", kind: "select", get: (c) => c.dre_group, options: DRE_OPTIONS, editable: true },
    { key: "system", label: "Do sistema", kind: "boolean", get: (c) => !!c.system_key },
    { key: "active", label: "Ativa", kind: "boolean", get: (c) => c.active, editable: (c) => !c.system_key },
  ];
  async function onEdit(c: FinCategory, key: string, value: CellValue) {
    await financeApi.updateCategory(c.id, { [key]: value } as { name?: string; dre_group?: string; active?: boolean });
    await onChanged();
  }
  async function add(e: FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      await financeApi.createCategory(form);
      setForm({ ...form, name: "" });
      await onChanged();
    } catch (e2) {
      setError(err(e2, "Não foi possível criar."));
    }
  }
  return (
    <div className="stack">
      {error && <Notice tone="error">{error}</Notice>}
      <form className="row" onSubmit={add}>
        <input aria-label="Nova categoria" placeholder="Nova categoria" value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} style={{ width: "16rem" }} />
        <select aria-label="Tipo" value={form.kind} onChange={(e) => setForm({ ...form, kind: e.target.value, dre_group: e.target.value === "receita" ? "outras_receitas" : "despesas_operacionais" })} style={{ width: "auto" }}>
          <option value="despesa">Despesa</option>
          <option value="receita">Receita</option>
        </select>
        <select aria-label="Linha da DRE" value={form.dre_group} onChange={(e) => setForm({ ...form, dre_group: e.target.value })} style={{ width: "auto" }}>
          {DRE_OPTIONS.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
        </select>
        <button className="btn btn--primary" disabled={!form.name.trim()}>Adicionar</button>
      </form>
      <DataGrid rows={categories} columns={columns} rowKey={(c) => c.id} onEdit={onEdit} rowClass={(c) => (c.active ? undefined : "is-muted")} exportName="Categorias financeiras" />
      <p className="muted small">A “Linha da DRE” define onde cada categoria aparece no demonstrativo de resultado.</p>
    </div>
  );
}
