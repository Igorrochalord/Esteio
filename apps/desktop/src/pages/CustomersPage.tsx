import { useMemo, useState, type FormEvent } from "react";

import { DataGrid, type CellValue, type GridColumn } from "../components/DataGrid";
import { Dialog } from "../components/Dialog";
import { Field } from "../components/Field";
import { Notice } from "../components/Notice";
import { useAsync } from "../hooks/useAsync";
import { useAuth } from "../hooks/useAuth";
import { ApiError } from "../services/api";
import { customersApi, type CustomerInput } from "../services/endpoints";
import { formatDocument, UF_OPTIONS } from "../services/labels";
import type { Customer } from "../services/types";

type EditableKey = "name" | "document" | "phone" | "email" | "city" | "state" | "notes";

export function CustomersPage() {
  const { can } = useAuth();
  const canEdit = can("customers", "write");
  const { data, error, loading, setData } = useAsync(customersApi.list);
  const [showInactive, setShowInactive] = useState(false);
  const [creating, setCreating] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const rows = useMemo(() => (data ?? []).filter((c) => showInactive || c.active), [data, showInactive]);

  const columns: GridColumn<Customer>[] = [
    { key: "name", label: "Nome", get: (c) => c.name, editable: canEdit, width: 220 },
    {
      key: "document",
      label: "CPF/CNPJ",
      get: (c) => c.document,
      render: (c) => formatDocument(c.document) || <span className="grid-blank">—</span>,
      editable: canEdit,
      nullable: true,
    },
    { key: "phone", label: "Telefone", get: (c) => c.phone, editable: canEdit, nullable: true },
    { key: "email", label: "E-mail", get: (c) => c.email, editable: canEdit, nullable: true },
    { key: "city", label: "Cidade", get: (c) => c.city, editable: canEdit, nullable: true },
    { key: "state", label: "UF", kind: "select", get: (c) => c.state, options: UF_OPTIONS, editable: canEdit, nullable: true, width: 60 },
    { key: "notes", label: "Observações", get: (c) => c.notes, editable: canEdit, nullable: true },
    { key: "created_at", label: "Cadastro", kind: "date", get: (c) => c.created_at },
    { key: "active", label: "Ativo", kind: "boolean", get: (c) => c.active, editable: canEdit },
  ];

  async function onEdit(c: Customer, key: string, value: CellValue) {
    const patch: Partial<CustomerInput & { active: boolean }> =
      key === "active" ? { active: !!value } : { [key as EditableKey]: value == null || value === "" ? null : String(value) };
    const saved = await customersApi.update(c.id, patch);
    setData((list) => (list ?? []).map((x) => (x.id === saved.id ? saved : x)));
  }

  return (
    <div className="page page--wide">
      <header className="page-header">
        <div>
          <h1>Clientes</h1>
          <p className="muted">Cadastre cidade e UF para os clientes aparecerem no mapa de vendas.</p>
        </div>
        {canEdit && (
          <button className="btn btn--primary" onClick={() => setCreating(true)}>
            Novo cliente
          </button>
        )}
      </header>
      {error && <Notice tone="error">{error}</Notice>}
      {notice && <Notice tone="success">{notice}</Notice>}
      {loading && !data ? (
        <p className="muted">Carregando…</p>
      ) : (
        <DataGrid
          rows={rows}
          columns={columns}
          rowKey={(c) => c.id}
          onEdit={canEdit ? onEdit : undefined}
          rowClass={(c) => (c.active ? undefined : "is-muted")}
          exportName="Clientes"
          emptyText="Nenhum cliente cadastrado."
          initialSort={{ key: "name", dir: "asc" }}
          toolbar={
            <label className="check">
              <input type="checkbox" checked={showInactive} onChange={(e) => setShowInactive(e.target.checked)} />
              Mostrar inativos
            </label>
          }
        />
      )}
      <Dialog wide open={creating} title="Novo cliente" onClose={() => setCreating(false)}>
        {creating && (
          <CustomerForm
            onDone={(c) => {
              setData((list) => [...(list ?? []), c]);
              setNotice(`${c.name} cadastrado.`);
              setCreating(false);
            }}
          />
        )}
      </Dialog>
    </div>
  );
}

export function CustomerForm({ onDone }: { onDone: (c: Customer) => void }) {
  const [form, setForm] = useState({ name: "", document: "", phone: "", email: "", city: "", state: "", notes: "" });
  const [error, setError] = useState<string | null>(null);
  const [fields, setFields] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const set = (k: keyof typeof form) => (e: { target: { value: string } }) => setForm({ ...form, [k]: e.target.value });

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    setFields({});
    const opt = (v: string) => (v.trim() ? v : null);
    try {
      onDone(
        await customersApi.create({
          name: form.name,
          document: opt(form.document),
          phone: opt(form.phone),
          email: opt(form.email),
          city: opt(form.city),
          state: opt(form.state),
          notes: opt(form.notes),
        }),
      );
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Não foi possível cadastrar.");
      if (err instanceof ApiError) setFields(err.fields);
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="stack" onSubmit={submit}>
      {error && <Notice tone="error">{error}</Notice>}
      <div className="form-grid">
        <Field label="Nome" required minLength={2} autoFocus value={form.name} error={fields.name} onChange={set("name")} />
        <Field label="CPF ou CNPJ" value={form.document} error={fields.document} onChange={set("document")} />
        <Field label="Telefone" type="tel" value={form.phone} onChange={set("phone")} />
        <Field label="E-mail" type="email" value={form.email} error={fields.email} onChange={set("email")} />
        <Field label="Cidade" value={form.city} onChange={set("city")} />
        <div className="field">
          <label htmlFor="cf-uf">UF</label>
          <select id="cf-uf" value={form.state} onChange={set("state")}>
            <option value="">—</option>
            {UF_OPTIONS.map((u) => (
              <option key={u.value} value={u.value}>{u.label}</option>
            ))}
          </select>
          {fields.state && <p className="field-error">{fields.state}</p>}
        </div>
      </div>
      <Field label="Observações" value={form.notes} onChange={set("notes")} />
      <div className="row row--end">
        <button className="btn btn--primary" disabled={busy}>{busy ? "Salvando…" : "Cadastrar cliente"}</button>
      </div>
    </form>
  );
}
