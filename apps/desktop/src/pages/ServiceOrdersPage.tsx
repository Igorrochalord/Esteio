import { CalendarClock, ClipboardList, Columns3, List, Printer, Receipt, Wrench, XCircle } from "lucide-react";
import { useEffect, useMemo, useState, type DragEvent, type FormEvent, type KeyboardEvent } from "react";
import { createPortal } from "react-dom";

import { DataGrid, type GridColumn } from "../components/DataGrid";
import { Dialog } from "../components/Dialog";
import { Field } from "../components/Field";
import { Notice } from "../components/Notice";
import { useAsync } from "../hooks/useAsync";
import { useAuth } from "../hooks/useAuth";
import { ApiError } from "../services/api";
import { customersApi, osApi, productsApi, salesApi } from "../services/endpoints";
import { date, dateTime, decimalInput, isoDay, money, parseDecimal, qty, toNum, tzOffset } from "../services/format";
import { OS_STATUS_LABEL, PAYMENT_LABEL } from "../services/labels";
import type { Customer, OsItem, OsStatus, PaymentMethod, Product, ServiceOrder } from "../services/types";
import { CustomerForm } from "./CustomersPage";

// PLANO: ordens de serviço fazem parte do Starter em diante (hoje liberado para todos).

const WORK: OsStatus[] = ["aberta", "em_andamento", "aguardando", "concluida"];
const PAYMENTS: PaymentMethod[] = ["dinheiro", "pix", "debito", "credito", "fiado", "outro"];
const VIEW_KEY = "nucleo.os.view";

const STATUS_TAG: Record<OsStatus, string> = {
  aberta: "tag",
  em_andamento: "tag tag--info",
  aguardando: "tag tag--warn",
  concluida: "tag tag--ok",
  faturada: "tag tag--ok",
  cancelada: "tag tag--danger",
};

export function StatusTag({ status }: { status: OsStatus }) {
  return <span className={STATUS_TAG[status]}>{OS_STATUS_LABEL[status]}</span>;
}

const isLate = (o: ServiceOrder, today: string) =>
  !!o.due_date && o.due_date < today && (o.status === "aberta" || o.status === "em_andamento" || o.status === "aguardando");

export function ServiceOrdersPage() {
  const { can } = useAuth();
  const canWrite = can("service_orders", "write");
  const [view, setView] = useState<"quadro" | "lista">(() => {
    try {
      return localStorage.getItem(VIEW_KEY) === "lista" ? "lista" : "quadro";
    } catch {
      return "quadro";
    }
  });
  const [status, setStatus] = useState("");
  const [creating, setCreating] = useState(false);
  const [openId, setOpenId] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const { data, error, loading, reload, setData } = useAsync(() => osApi.list());
  const today = isoDay(new Date());

  const changeView = (v: "quadro" | "lista") => {
    setView(v);
    try {
      localStorage.setItem(VIEW_KEY, v);
    } catch {
      /* ignora */
    }
  };

  async function move(id: string, to: OsStatus) {
    const cur = data?.find((o) => o.id === id);
    if (!cur || cur.status === to) return;
    setActionError(null);
    // otimista: o cartão muda de coluna na hora
    setData((list) => (list ?? []).map((o) => (o.id === id ? { ...o, status: to } : o)));
    try {
      const o = await osApi.changeStatus(id, to);
      setData((list) => (list ?? []).map((x) => (x.id === id ? { ...o, items: null, events: null } : x)));
    } catch (err) {
      setActionError(err instanceof ApiError ? err.message : "Não foi possível mudar a situação.");
      await reload();
    }
  }

  const rows = useMemo(() => (data ?? []).filter((o) => !status || o.status === status), [data, status]);
  const columns: GridColumn<ServiceOrder>[] = [
    { key: "number", label: "Nº", kind: "number", get: (o) => o.number, width: 60 },
    { key: "created", label: "Abertura", kind: "datetime", get: (o) => o.created_at },
    { key: "customer", label: "Cliente", get: (o) => o.customer_name },
    { key: "equipment", label: "Equipamento / serviço", get: (o) => o.equipment ?? o.reported },
    { key: "tech", label: "Técnico", get: (o) => o.technician_name ?? "—" },
    {
      key: "due",
      label: "Previsão",
      get: (o) => o.due_date,
      render: (o) => <span className={isLate(o, today) ? "neg" : undefined}>{date(o.due_date ? `${o.due_date}T12:00:00` : null)}</span>,
    },
    { key: "total", label: "Total", kind: "money", get: (o) => o.total, total: "sum" },
    { key: "status", label: "Situação", get: (o) => OS_STATUS_LABEL[o.status], render: (o) => <StatusTag status={o.status} /> },
  ];

  return (
    <div className="page page--wide">
      <header className="page-header">
        <div>
          <h1>Ordens de serviço</h1>
          <p className="muted">Do pedido do cliente ao faturamento: serviços e peças, técnico, prazos e histórico de cada OS.</p>
        </div>
        {canWrite && (
          <button className="btn btn--primary btn--large" onClick={() => setCreating(true)}>
            Nova OS
          </button>
        )}
      </header>

      <div className="filters no-print">
        <div className="seg" role="radiogroup" aria-label="Visualização">
          <button type="button" role="radio" aria-checked={view === "quadro"} className={view === "quadro" ? "is-on" : undefined} onClick={() => changeView("quadro")}>
            <Columns3 size={16} aria-hidden /> Quadro
          </button>
          <button type="button" role="radio" aria-checked={view === "lista"} className={view === "lista" ? "is-on" : undefined} onClick={() => changeView("lista")}>
            <List size={16} aria-hidden /> Lista
          </button>
        </div>
      </div>

      {error && <Notice tone="error">{error}</Notice>}
      {actionError && <Notice tone="error">{actionError}</Notice>}
      {notice && <Notice tone="success">{notice}</Notice>}

      {loading && !data ? (
        <p className="muted">Carregando…</p>
      ) : view === "quadro" ? (
        <Board orders={data ?? []} today={today} canMove={canWrite} onOpen={setOpenId} onMove={move} />
      ) : (
        <DataGrid
          rows={rows}
          columns={columns}
          rowKey={(o) => o.id}
          onRowOpen={(o) => setOpenId(o.id)}
          rowClass={(o) => (o.status === "cancelada" ? "is-canceled" : undefined)}
          exportName="Ordens de serviço"
          emptyText="Nenhuma ordem de serviço."
          initialSort={{ key: "number", dir: "desc" }}
          toolbar={
            <select aria-label="Situação" value={status} onChange={(e) => setStatus(e.target.value)}>
              <option value="">Todas</option>
              {Object.entries(OS_STATUS_LABEL).map(([k, v]) => (
                <option key={k} value={k}>{v}</option>
              ))}
            </select>
          }
        />
      )}

      <Dialog wide open={creating} title="Nova ordem de serviço" onClose={() => setCreating(false)}>
        {creating && (
          <NewOsForm
            onDone={async (o) => {
              setCreating(false);
              setNotice(`OS nº ${o.number} aberta para ${o.customer_name}. Adicione serviços e peças no detalhe.`);
              await reload();
              setOpenId(o.id);
            }}
          />
        )}
      </Dialog>
      <Dialog wide open={!!openId} title="Ordem de serviço" onClose={() => setOpenId(null)}>
        {openId && (
          <OsDetail
            id={openId}
            onChanged={async (o, message) => {
              if (message) setNotice(message);
              setData((list) => (list ?? []).map((x) => (x.id === o.id ? { ...o, items: null, events: null } : x)));
            }}
          />
        )}
      </Dialog>
    </div>
  );
}

// ------------------------------------------------------------------ quadro

function Board({
  orders,
  today,
  canMove,
  onOpen,
  onMove,
}: {
  orders: ServiceOrder[];
  today: string;
  canMove: boolean;
  onOpen: (id: string) => void;
  onMove: (id: string, to: OsStatus) => void;
}) {
  const [over, setOver] = useState<OsStatus | null>(null);
  const recentInvoiced = orders.filter((o) => o.status === "faturada").length;

  const drop = (to: OsStatus) => (e: DragEvent) => {
    e.preventDefault();
    setOver(null);
    const id = e.dataTransfer.getData("text/os-id");
    if (id) onMove(id, to);
  };

  return (
    <>
      <div className="os-board">
        {WORK.map((s) => {
          const list = orders.filter((o) => o.status === s).sort((a, b) => (a.due_date ?? "9999").localeCompare(b.due_date ?? "9999"));
          const sum = list.reduce((acc, o) => acc + toNum(o.total), 0);
          return (
            <section
              key={s}
              className={over === s ? "os-col is-over" : "os-col"}
              aria-label={OS_STATUS_LABEL[s]}
              onDragOver={(e) => {
                if (!canMove) return;
                e.preventDefault();
                setOver(s);
              }}
              onDragLeave={() => setOver((o) => (o === s ? null : o))}
              onDrop={drop(s)}
            >
              <header className="os-col-head">
                <StatusTag status={s} />
                <span className="muted small">{list.length} · {money(sum)}</span>
              </header>
              <ul className="os-cards">
                {list.map((o) => (
                  <li key={o.id}>
                    <button
                      type="button"
                      className={isLate(o, today) ? "os-card is-late" : "os-card"}
                      draggable={canMove}
                      onDragStart={(e) => {
                        e.dataTransfer.setData("text/os-id", o.id);
                        e.dataTransfer.effectAllowed = "move";
                      }}
                      onClick={() => onOpen(o.id)}
                    >
                      <span className="os-card-top">
                        <strong>OS {o.number}</strong>
                        <span className="num">{money(o.total)}</span>
                      </span>
                      <span className="os-card-customer">{o.customer_name}</span>
                      <span className="muted small os-card-what">{o.equipment ?? o.reported}</span>
                      <span className="os-card-foot small">
                        {o.technician_name && (
                          <span><Wrench size={13} aria-hidden /> {o.technician_name.split(" ")[0]}</span>
                        )}
                        {o.due_date && (
                          <span className={isLate(o, today) ? "neg" : "muted"}>
                            <CalendarClock size={13} aria-hidden /> {isLate(o, today) ? "Atrasada · " : ""}{date(`${o.due_date}T12:00:00`)}
                          </span>
                        )}
                      </span>
                    </button>
                  </li>
                ))}
                {!list.length && <li className="os-empty muted small">Nenhuma OS</li>}
              </ul>
            </section>
          );
        })}
      </div>
      <p className="muted small">
        {canMove ? "Arraste um cartão para mudar a situação, ou abra a OS. " : ""}
        {recentInvoiced > 0 && (recentInvoiced === 1 ? "A OS faturada está na visualização em lista." : `As ${recentInvoiced} OS faturadas estão na visualização em lista.`)}
      </p>
    </>
  );
}

// ------------------------------------------------------------------ nova OS

function NewOsForm({ onDone }: { onDone: (o: ServiceOrder) => Promise<void> }) {
  const { can } = useAuth();
  const { data: customers, setData: setCustomers } = useAsync(() =>
    can("customers", "read") ? customersApi.list() : Promise.resolve([] as Customer[]),
  );
  const { data: techs } = useAsync(salesApi.sellers);
  const [customerId, setCustomerId] = useState("");
  const [equipment, setEquipment] = useState("");
  const [reported, setReported] = useState("");
  const [technicianId, setTechnicianId] = useState("");
  const [scheduled, setScheduled] = useState("");
  const [due, setDue] = useState("");
  const [fields, setFields] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [newCustomer, setNewCustomer] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!customerId) return setFields({ customer_id: "Escolha o cliente" });
    setBusy(true);
    setError(null);
    setFields({});
    try {
      const o = await osApi.create({
        customer_id: customerId,
        equipment: equipment.trim() || null,
        reported,
        technician_id: technicianId || null,
        scheduled_date: scheduled || null,
        due_date: due || null,
        items: [],
      });
      await onDone(o);
    } catch (err) {
      if (err instanceof ApiError) {
        setFields(err.fields);
        setError(err.message);
      } else setError("Não foi possível abrir a OS.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="stack" onSubmit={submit}>
      {error && <Notice tone="error">{error}</Notice>}
      <div className="field">
        <label htmlFor="os-customer">Cliente</label>
        <div className="row row--fill">
          <select id="os-customer" value={customerId} onChange={(e) => setCustomerId(e.target.value)} aria-invalid={!!fields.customer_id}>
            <option value="">Escolha…</option>
            {(customers ?? []).filter((c) => c.active).map((c) => (
              <option key={c.id} value={c.id}>{c.name}{c.phone ? ` · ${c.phone}` : ""}</option>
            ))}
          </select>
          {can("customers", "write") && (
            <button type="button" className="btn btn--ghost" onClick={() => setNewCustomer(true)} aria-label="Novo cliente">+</button>
          )}
        </div>
        {fields.customer_id && <p className="field-error">{fields.customer_id}</p>}
      </div>
      <Field label="Equipamento ou objeto (opcional)" placeholder="Ex.: Notebook Dell, placa ABC-1234, ar-condicionado da sala" value={equipment} onChange={(e) => setEquipment(e.target.value)} />
      <div className="field">
        <label htmlFor="os-reported">Problema relatado / pedido do cliente</label>
        <textarea id="os-reported" rows={3} required value={reported} onChange={(e) => setReported(e.target.value)} aria-invalid={!!fields.reported} />
        {fields.reported && <p className="field-error">{fields.reported}</p>}
      </div>
      <div className="form-grid">
        <div className="field">
          <label htmlFor="os-tech">Técnico responsável</label>
          <select id="os-tech" value={technicianId} onChange={(e) => setTechnicianId(e.target.value)}>
            <option value="">A definir</option>
            {(techs ?? []).map((t) => (
              <option key={t.id} value={t.id}>{t.name}</option>
            ))}
          </select>
        </div>
        <Field label="Agendada para" type="date" value={scheduled} onChange={(e) => setScheduled(e.target.value)} error={fields.scheduled_date} />
        <Field label="Previsão de entrega" type="date" value={due} onChange={(e) => setDue(e.target.value)} error={fields.due_date} />
      </div>
      <div className="row row--end">
        <button className="btn btn--primary" disabled={busy}>{busy ? "Abrindo…" : "Abrir OS"}</button>
      </div>

      <Dialog wide open={newCustomer} title="Novo cliente" onClose={() => setNewCustomer(false)}>
        {newCustomer && (
          <CustomerForm
            onDone={(c) => {
              setCustomers((list) => [...(list ?? []), c]);
              setCustomerId(c.id);
              setNewCustomer(false);
            }}
          />
        )}
      </Dialog>
    </form>
  );
}

// ------------------------------------------------------------------ detalhe

interface Line {
  productId: string;
  name: string;
  isService: boolean;
  unit: string;
  qty: string;
  unitPrice: string;
  discount: string;
}

function lineCents(l: Line): number {
  const q = Math.round(toNum(parseDecimal(l.qty) ?? 0) * 1000);
  const p = Math.round(toNum(parseDecimal(l.unitPrice) ?? 0) * 100);
  const d = Math.round(toNum(parseDecimal(l.discount) ?? 0) * 100);
  return Math.floor((q * p + 500) / 1000) - d;
}

const toLine = (i: OsItem, products: Product[]): Line => {
  const p = products.find((x) => x.id === i.product_id);
  return {
    productId: i.product_id,
    name: i.description,
    isService: i.is_service,
    unit: p?.unit ?? "",
    qty: decimalInput(i.qty),
    unitPrice: decimalInput(i.unit_price),
    discount: toNum(i.discount) ? decimalInput(i.discount) : "",
  };
};

function OsDetail({ id, onChanged }: { id: string; onChanged: (o: ServiceOrder, message?: string) => Promise<void> }) {
  const { can } = useAuth();
  const canWrite = can("service_orders", "write");
  const { data: os, error, setData } = useAsync(() => osApi.get(id), [id]);
  const [actionError, setActionError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (error) return <Notice tone="error">{error}</Notice>;
  if (!os) return <p className="muted">Carregando…</p>;

  const editable = canWrite && os.status !== "faturada" && os.status !== "cancelada";

  async function run(fn: () => Promise<ServiceOrder>, message?: string) {
    setBusy(true);
    setActionError(null);
    try {
      const o = await fn();
      setData(o);
      await onChanged(o, message);
      return true;
    } catch (err) {
      setActionError(err instanceof ApiError ? err.message : "Não foi possível concluir a ação.");
      return false;
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="stack os-detail">
      <div className="kv-grid">
        <div><span>OS</span><strong>nº {os.number}</strong></div>
        <div><span>Cliente</span><strong>{os.customer_name}</strong>{os.customer_phone && <span>{os.customer_phone}</span>}</div>
        <div><span>Aberta em</span><strong>{dateTime(os.created_at)}</strong></div>
        <div><span>Situação</span><strong><StatusTag status={os.status} /></strong></div>
        <div><span>Total</span><strong>{money(os.total)}</strong></div>
      </div>

      {os.status === "faturada" && <Notice tone="success">Faturada na venda nº {os.sale_number}. Para desfazer, cancele essa venda em Vendas.</Notice>}
      {os.status === "cancelada" && <Notice tone="error">Cancelada: {os.canceled_reason}</Notice>}
      {actionError && <Notice tone="error">{actionError}</Notice>}

      {editable && (
        <div className="os-steps" role="group" aria-label="Mudar situação">
          {WORK.map((s, i) => (
            <button
              key={s}
              type="button"
              disabled={busy}
              className={os.status === s ? "os-step is-on" : WORK.indexOf(os.status) > i ? "os-step is-done" : "os-step"}
              aria-pressed={os.status === s}
              onClick={() => void run(() => osApi.changeStatus(os.id, s), `OS nº ${os.number}: ${OS_STATUS_LABEL[s].toLowerCase()}.`)}
            >
              <span className="os-step-dot" aria-hidden>{i + 1}</span>
              {OS_STATUS_LABEL[s]}
            </button>
          ))}
        </div>
      )}

      <InfoForm os={os} editable={editable} busy={busy} onSave={(patch) => run(() => osApi.update(os.id, patch), "Dados da OS salvos.")} />
      <ItemsEditor os={os} editable={editable} busy={busy} onSave={(items) => run(() => osApi.setItems(os.id, items), "Serviços e peças salvos.")} />
      <Timeline os={os} canWrite={canWrite} busy={busy} onNote={(note) => run(() => osApi.addNote(os.id, note))} />

      <Actions os={os} busy={busy} run={run} />
      <OsPrint os={os} />
    </div>
  );
}

function InfoForm({
  os,
  editable,
  busy,
  onSave,
}: {
  os: ServiceOrder;
  editable: boolean;
  busy: boolean;
  onSave: (patch: Record<string, unknown>) => Promise<boolean>;
}) {
  const { data: techs } = useAsync(() => (editable ? salesApi.sellers() : Promise.resolve([])), [editable]);
  const [equipment, setEquipment] = useState(os.equipment ?? "");
  const [reported, setReported] = useState(os.reported);
  const [diagnosis, setDiagnosis] = useState(os.diagnosis ?? "");
  const [technicianId, setTechnicianId] = useState(os.technician_id ?? "");
  const [scheduled, setScheduled] = useState(os.scheduled_date ?? "");
  const [due, setDue] = useState(os.due_date ?? "");
  const [discount, setDiscount] = useState(toNum(os.discount) ? decimalInput(os.discount) : "");
  const [error, setError] = useState<string | null>(null);

  if (!editable) {
    return (
      <section className="panel panel--quiet">
        <h2>Atendimento</h2>
        <dl className="os-facts">
          <div><dt>Equipamento</dt><dd>{os.equipment ?? "—"}</dd></div>
          <div><dt>Relato</dt><dd>{os.reported}</dd></div>
          <div><dt>Diagnóstico e solução</dt><dd>{os.diagnosis ?? "—"}</dd></div>
          <div><dt>Técnico</dt><dd>{os.technician_name ?? "—"}</dd></div>
          <div><dt>Previsão</dt><dd>{date(os.due_date ? `${os.due_date}T12:00:00` : null)}</dd></div>
        </dl>
      </section>
    );
  }

  async function save(e: FormEvent) {
    e.preventDefault();
    if (discount.trim() && parseDecimal(discount) == null) return setError("Desconto inválido.");
    setError(null);
    await onSave({
      equipment: equipment.trim() || null,
      reported,
      diagnosis: diagnosis.trim() || null,
      technician_id: technicianId || null,
      scheduled_date: scheduled || null,
      due_date: due || null,
      discount: discount.trim() ? parseDecimal(discount) : "0",
    });
  }

  return (
    <form className="panel panel--quiet stack" onSubmit={save}>
      <h2>Atendimento</h2>
      {error && <Notice tone="error">{error}</Notice>}
      <Field label="Equipamento ou objeto" value={equipment} onChange={(e) => setEquipment(e.target.value)} />
      <div className="form-grid form-grid--2">
        <div className="field">
          <label htmlFor="osd-reported">Problema relatado</label>
          <textarea id="osd-reported" rows={3} required value={reported} onChange={(e) => setReported(e.target.value)} />
        </div>
        <div className="field">
          <label htmlFor="osd-diag">Diagnóstico e solução</label>
          <textarea id="osd-diag" rows={3} value={diagnosis} onChange={(e) => setDiagnosis(e.target.value)} />
        </div>
      </div>
      <div className="form-grid">
        <div className="field">
          <label htmlFor="osd-tech">Técnico</label>
          <select id="osd-tech" value={technicianId} onChange={(e) => setTechnicianId(e.target.value)}>
            <option value="">A definir</option>
            {(techs ?? []).map((t) => (
              <option key={t.id} value={t.id}>{t.name}</option>
            ))}
            {os.technician_id && !(techs ?? []).some((t) => t.id === os.technician_id) && (
              <option value={os.technician_id}>{os.technician_name}</option>
            )}
          </select>
        </div>
        <Field label="Agendada para" type="date" value={scheduled} onChange={(e) => setScheduled(e.target.value)} />
        <Field label="Previsão de entrega" type="date" value={due} onChange={(e) => setDue(e.target.value)} />
        <Field label="Desconto na OS (R$)" inputMode="decimal" placeholder="0,00" value={discount} onChange={(e) => setDiscount(e.target.value)} />
      </div>
      <div className="row row--end">
        <button className="btn btn--ghost" disabled={busy}>Salvar dados</button>
      </div>
    </form>
  );
}

function ItemsEditor({
  os,
  editable,
  busy,
  onSave,
}: {
  os: ServiceOrder;
  editable: boolean;
  busy: boolean;
  onSave: (items: { product_id: string; description: string; qty: string; unit_price: string; discount: string | null }[]) => Promise<boolean>;
}) {
  const { data: products } = useAsync(() => (editable ? productsApi.list() : Promise.resolve([] as Product[])), [editable]);
  const [lines, setLines] = useState<Line[] | null>(null);
  const [search, setSearch] = useState("");
  const [highlight, setHighlight] = useState(0);
  const [error, setError] = useState<string | null>(null);

  // linhas editáveis nascem dos itens salvos (e voltam a eles depois de salvar)
  useEffect(() => {
    setLines((os.items ?? []).map((i) => toLine(i, products ?? [])));
  }, [os.items, products]);

  const active = useMemo(() => (products ?? []).filter((p) => p.active), [products]);
  const matches = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return [];
    return active.filter((p) => p.code.toLowerCase().includes(q) || p.barcode === q || p.name.toLowerCase().includes(q)).slice(0, 8);
  }, [active, search]);

  const saved = JSON.stringify((os.items ?? []).map((i) => toLine(i, products ?? [])));
  const dirty = lines != null && JSON.stringify(lines) !== saved;

  function add(p: Product) {
    setLines((ls) => [
      ...(ls ?? []),
      { productId: p.id, name: p.name, isService: !p.track_stock, unit: p.unit, qty: "1", unitPrice: decimalInput(p.price), discount: "" },
    ]);
    setSearch("");
    setHighlight(0);
  }

  function onKey(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlight((h) => Math.min(h + 1, matches.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlight((h) => Math.max(h - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const exact = active.find((p) => p.barcode === search.trim() || p.code.toLowerCase() === search.trim().toLowerCase());
      const pick = exact ?? matches[highlight];
      if (pick) add(pick);
    }
  }

  async function save() {
    if (!lines) return;
    const items = [];
    for (const l of lines) {
      const q = parseDecimal(l.qty);
      const price = parseDecimal(l.unitPrice);
      const d = l.discount.trim() ? parseDecimal(l.discount) : null;
      if (!q || toNum(q) <= 0 || price == null || (l.discount.trim() && d == null)) {
        return setError(`Confira a quantidade, o preço e o desconto de "${l.name}".`);
      }
      items.push({ product_id: l.productId, description: l.name, qty: q, unit_price: price, discount: d });
    }
    setError(null);
    await onSave(items);
  }

  const setLine = (i: number, patch: Partial<Line>) => setLines((ls) => (ls ?? []).map((l, j) => (j === i ? { ...l, ...patch } : l)));
  const list = lines ?? [];
  const subtotal = list.reduce((acc, l) => acc + lineCents(l), 0);
  const discountCents = Math.round(toNum(os.discount) * 100);

  return (
    <section className="panel panel--quiet stack">
      <h2>Serviços e peças</h2>
      {error && <Notice tone="error">{error}</Notice>}
      {editable && (
        <div className="sale-search">
          <input
            type="search"
            placeholder="Adicionar serviço ou peça: nome, código ou código de barras…"
            aria-label="Adicionar serviço ou peça"
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setHighlight(0);
            }}
            onKeyDown={onKey}
          />
          {matches.length > 0 && (
            <ul className="sale-suggest" role="listbox">
              {matches.map((p, i) => (
                <li key={p.id} role="option" aria-selected={i === highlight}>
                  <button type="button" onMouseDown={(e) => e.preventDefault()} onClick={() => add(p)}>
                    <span>
                      <span className="muted">{p.code}</span> {p.name} {!p.track_stock && <span className="tag">serviço</span>}
                    </span>
                    <span className="num">
                      {money(p.price)}
                      {p.track_stock && <span className={p.below_min ? "stock-low" : "muted"}> · {qty(p.stock)} {p.unit}</span>}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
      <div className="table-wrap">
        <table className="table sale-items">
          <thead>
            <tr>
              <th>Descrição</th>
              <th className="num">Qtd.</th>
              <th className="num">Preço un.</th>
              <th className="num">Desconto</th>
              <th className="num">Total</th>
              {editable && <th aria-label="Remover" />}
            </tr>
          </thead>
          <tbody>
            {list.map((l, i) => (
              <tr key={`${l.productId}-${i}`}>
                <td>
                  {editable ? (
                    <input aria-label="Descrição" className="cell-input cell-input--text" value={l.name} onChange={(e) => setLine(i, { name: e.target.value })} />
                  ) : (
                    l.name
                  )}{" "}
                  {l.isService ? <span className="tag">serviço</span> : <span className="tag">peça</span>}
                </td>
                <td className="num">
                  {editable ? (
                    <input aria-label={`Quantidade de ${l.name}`} inputMode="decimal" className="cell-input" value={l.qty} onChange={(e) => setLine(i, { qty: e.target.value })} />
                  ) : (
                    `${qty(parseDecimal(l.qty))} ${l.unit}`
                  )}
                </td>
                <td className="num">
                  {editable ? (
                    <input aria-label={`Preço de ${l.name}`} inputMode="decimal" className="cell-input" value={l.unitPrice} onChange={(e) => setLine(i, { unitPrice: e.target.value })} />
                  ) : (
                    money(parseDecimal(l.unitPrice))
                  )}
                </td>
                <td className="num">
                  {editable ? (
                    <input aria-label={`Desconto em ${l.name}`} inputMode="decimal" className="cell-input" placeholder="0,00" value={l.discount} onChange={(e) => setLine(i, { discount: e.target.value })} />
                  ) : (
                    money(parseDecimal(l.discount) ?? 0)
                  )}
                </td>
                <td className="num">{money(lineCents(l) / 100)}</td>
                {editable && (
                  <td>
                    <button type="button" className="btn btn--ghost" aria-label={`Remover ${l.name}`} onClick={() => setLines((ls) => (ls ?? []).filter((_, j) => j !== i))}>
                      ✕
                    </button>
                  </td>
                )}
              </tr>
            ))}
            {!list.length && (
              <tr>
                <td colSpan={editable ? 6 : 5} className="grid-empty">
                  {editable ? "Busque acima e adicione a mão de obra e as peças usadas." : "Sem itens."}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      <div className="row os-items-foot">
        <dl className="sale-totals">
          <div><dt>Subtotal</dt><dd>{money(subtotal / 100)}</dd></div>
          {discountCents > 0 && <div><dt>Desconto na OS</dt><dd>− {money(discountCents / 100)}</dd></div>}
          <div className="sale-total"><dt>Total</dt><dd>{money(Math.max(subtotal - discountCents, 0) / 100)}</dd></div>
        </dl>
        {editable && (
          <button type="button" className="btn btn--primary" disabled={busy || !dirty} onClick={() => void save()}>
            {dirty ? "Salvar itens" : "Itens salvos"}
          </button>
        )}
      </div>
    </section>
  );
}

function eventText(e: NonNullable<ServiceOrder["events"]>[number]): string {
  switch (e.kind) {
    case "criada":
      return "OS aberta";
    case "faturada":
      return "Faturada (venda gerada)";
    case "nota":
      return "Anotação";
    default:
      return e.to_status === "cancelada" ? "Cancelada" : `${OS_STATUS_LABEL[e.from_status ?? ""] ?? "—"} → ${OS_STATUS_LABEL[e.to_status ?? ""] ?? "—"}`;
  }
}

function Timeline({ os, canWrite, busy, onNote }: { os: ServiceOrder; canWrite: boolean; busy: boolean; onNote: (note: string) => Promise<boolean> }) {
  const [note, setNote] = useState("");
  const events = [...(os.events ?? [])].reverse();
  return (
    <section className="panel panel--quiet stack">
      <h2>Linha do tempo</h2>
      {canWrite && (
        <form
          className="row row--fill no-print"
          onSubmit={async (e) => {
            e.preventDefault();
            if (note.trim() && (await onNote(note))) setNote("");
          }}
        >
          <input aria-label="Nova anotação" placeholder="Anotar: peça encomendada, cliente avisado…" value={note} onChange={(e) => setNote(e.target.value)} />
          <button className="btn btn--ghost" disabled={busy || !note.trim()}>Anotar</button>
        </form>
      )}
      <ol className="os-timeline">
        {events.map((e) => (
          <li key={e.id} className={`os-ev os-ev--${e.kind}`}>
            <span className="os-ev-dot" aria-hidden />
            <div>
              <strong>{eventText(e)}</strong>
              {e.note && <p>{e.note}</p>}
              <span className="muted small">{dateTime(e.created_at)}{e.user_name ? ` · ${e.user_name}` : ""}</span>
            </div>
          </li>
        ))}
      </ol>
    </section>
  );
}

function Actions({ os, busy, run }: { os: ServiceOrder; busy: boolean; run: (fn: () => Promise<ServiceOrder>, message?: string) => Promise<boolean> }) {
  const { can } = useAuth();
  const [mode, setMode] = useState<"none" | "invoice" | "cancel">("none");
  const [payment, setPayment] = useState<PaymentMethod>("pix");
  const [reason, setReason] = useState("");
  const canInvoice = can("service_orders", "write") && can("sales", "write") && os.status === "concluida";
  const canCancel = can("service_orders", "manage") && os.status !== "faturada" && os.status !== "cancelada";

  return (
    <div className="stack no-print">
      {mode === "invoice" && (
        <form
          className="panel stack"
          onSubmit={async (e) => {
            e.preventDefault();
            const ok = await run(
              () => osApi.invoice(os.id, { payment_method: payment, tz_offset_minutes: tzOffset() }),
              `OS nº ${os.number} faturada: ${money(os.total)} (${PAYMENT_LABEL[payment]}). Peças baixadas do estoque e valor lançado no financeiro.`,
            );
            if (ok) setMode("none");
          }}
        >
          <h2>Faturar {money(os.total)}</h2>
          <div className="pay-grid" role="radiogroup" aria-label="Pagamento">
            {PAYMENTS.map((p) => (
              <label key={p} className={payment === p ? "is-on" : undefined}>
                <input type="radio" name="os-payment" checked={payment === p} onChange={() => setPayment(p)} />
                {PAYMENT_LABEL[p]}
              </label>
            ))}
          </div>
          <p className="muted small">Gera uma venda com os itens da OS. À vista entra no caixa; fiado vira conta a receber em 30 dias.</p>
          <div className="row row--end">
            <button type="button" className="btn btn--ghost" onClick={() => setMode("none")}>Voltar</button>
            <button className="btn btn--primary" disabled={busy}>Confirmar faturamento</button>
          </div>
        </form>
      )}
      {mode === "cancel" && (
        <form
          className="row"
          onSubmit={async (e) => {
            e.preventDefault();
            const ok = await run(() => osApi.cancel(os.id, reason), `OS nº ${os.number} cancelada.`);
            if (ok) setMode("none");
          }}
        >
          <input aria-label="Motivo do cancelamento" placeholder="Motivo do cancelamento" required minLength={3} autoFocus value={reason} onChange={(e) => setReason(e.target.value)} />
          <button className="btn btn--primary" disabled={busy}>Confirmar cancelamento</button>
          <button type="button" className="btn btn--ghost" onClick={() => setMode("none")}>Voltar</button>
        </form>
      )}
      {mode === "none" && (
        <div className="row row--end">
          {canCancel && (
            <button type="button" className="btn btn--ghost" onClick={() => setMode("cancel")}>
              <XCircle size={16} aria-hidden /> Cancelar OS
            </button>
          )}
          <button type="button" className="btn btn--ghost" onClick={() => printOs()}>
            <Printer size={16} aria-hidden /> Imprimir / PDF
          </button>
          {canInvoice && (
            <button type="button" className="btn btn--primary" onClick={() => setMode("invoice")}>
              <Receipt size={16} aria-hidden /> Faturar
            </button>
          )}
        </div>
      )}
      {os.status !== "concluida" && os.status !== "faturada" && os.status !== "cancelada" && (
        <p className="muted small row"><ClipboardList size={14} aria-hidden /> Para faturar, marque a OS como concluída.</p>
      )}
    </div>
  );
}

// ------------------------------------------------------------------ impressão

/** Imprime só a folha da OS (o resto do app some enquanto imprime). */
function printOs() {
  document.body.classList.add("printing-os");
  const done = () => {
    document.body.classList.remove("printing-os");
    window.removeEventListener("afterprint", done);
  };
  window.addEventListener("afterprint", done);
  window.print();
}

function OsPrint({ os }: { os: ServiceOrder }) {
  const { me } = useAuth();
  const items = os.items ?? [];
  return createPortal(
    <article className="os-print" aria-hidden>
      <header className="os-print-head">
        <div>
          <strong className="os-print-company">{me?.company.name}</strong>
          <div>Ordem de serviço nº {os.number}</div>
        </div>
        <div className="os-print-meta">
          <div>Abertura: {dateTime(os.created_at)}</div>
          <div>Situação: {OS_STATUS_LABEL[os.status]}</div>
          {os.due_date && <div>Previsão: {date(`${os.due_date}T12:00:00`)}</div>}
        </div>
      </header>
      <section className="os-print-box">
        <div><b>Cliente:</b> {os.customer_name}{os.customer_phone ? ` · ${os.customer_phone}` : ""}</div>
        {os.equipment && <div><b>Equipamento:</b> {os.equipment}</div>}
        <div><b>Relato:</b> {os.reported}</div>
        {os.diagnosis && <div><b>Diagnóstico e solução:</b> {os.diagnosis}</div>}
        {os.technician_name && <div><b>Técnico:</b> {os.technician_name}</div>}
      </section>
      <table className="os-print-table">
        <thead>
          <tr><th>Descrição</th><th>Qtd.</th><th>Preço un.</th><th>Desconto</th><th>Total</th></tr>
        </thead>
        <tbody>
          {items.map((i) => (
            <tr key={i.id}>
              <td>{i.description}</td>
              <td>{qty(i.qty)}</td>
              <td>{money(i.unit_price)}</td>
              <td>{toNum(i.discount) ? money(i.discount) : "—"}</td>
              <td>{money(i.total)}</td>
            </tr>
          ))}
        </tbody>
        <tfoot>
          {toNum(os.discount) > 0 && (
            <tr><td colSpan={4}>Desconto na OS</td><td>− {money(os.discount)}</td></tr>
          )}
          <tr><td colSpan={4}><b>Total</b></td><td><b>{money(os.total)}</b></td></tr>
        </tfoot>
      </table>
      <div className="os-print-sign">
        <div>Assinatura do cliente</div>
        <div>Responsável técnico</div>
      </div>
    </article>,
    document.body,
  );
}
