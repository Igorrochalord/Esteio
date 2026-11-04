import { useEffect, useMemo, useRef, useState, type FormEvent, type KeyboardEvent } from "react";

import { DataGrid, type GridColumn } from "../components/DataGrid";
import { Dialog } from "../components/Dialog";
import { Field } from "../components/Field";
import { Notice } from "../components/Notice";
import { PeriodPicker } from "../components/PeriodPicker";
import { useAsync } from "../hooks/useAsync";
import { useAuth } from "../hooks/useAuth";
import { ApiError } from "../services/api";
import { customersApi, productsApi, salesApi } from "../services/endpoints";
import { dateTime, decimalInput, money, parseDecimal, qty, toNum } from "../services/format";
import { PAYMENT_LABEL } from "../services/labels";
import { periodBounds, presetPeriod, type Period } from "../services/period";
import type { Customer, PaymentMethod, Product, Sale, SaleItem } from "../services/types";
import { CustomerForm } from "./CustomersPage";

const PAYMENTS: PaymentMethod[] = ["dinheiro", "pix", "debito", "credito", "fiado", "outro"];

export function SalesPage() {
  const { can } = useAuth();
  const canSell = can("sales", "write");
  const [period, setPeriod] = useState<Period>(() => presetPeriod("month"));
  const [status, setStatus] = useState("");
  const [creating, setCreating] = useState(false);
  const [openSale, setOpenSale] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const { data, error, loading, reload } = useAsync(
    () => salesApi.list({ ...periodBounds(period), status: status || undefined, limit: 5000 }),
    [period.from, period.to, status],
  );
  const showProfit = (data ?? []).some((s) => s.profit != null);

  const columns: GridColumn<Sale>[] = [
    { key: "number", label: "Nº", kind: "number", get: (s) => s.number, width: 60 },
    { key: "sold_at", label: "Data", kind: "datetime", get: (s) => s.sold_at },
    { key: "customer", label: "Cliente", get: (s) => s.customer_name ?? "Consumidor" },
    { key: "seller", label: "Vendedor", get: (s) => s.seller_name },
    { key: "payment", label: "Pagamento", get: (s) => PAYMENT_LABEL[s.payment_method] ?? s.payment_method },
    { key: "items", label: "Itens", kind: "number", get: (s) => s.item_count },
    { key: "discount", label: "Desconto", kind: "money", get: (s) => s.discount, total: "sum" },
    { key: "total", label: "Total", kind: "money", get: (s) => s.total, total: "sum" },
    ...(showProfit
      ? ([{ key: "profit", label: "Lucro", kind: "money", get: (s) => s.profit, total: "sum" }] satisfies GridColumn<Sale>[])
      : []),
    {
      key: "status",
      label: "Situação",
      get: (s) => (s.status === "cancelada" ? "Cancelada" : "Concluída"),
      render: (s) =>
        s.status === "cancelada" ? <span className="tag tag--danger">Cancelada</span> : <span className="tag tag--ok">Concluída</span>,
    },
  ];

  return (
    <div className="page page--wide">
      <header className="page-header">
        <div>
          <h1>Vendas</h1>
          <p className="muted">Enter ou duplo clique numa venda abre os detalhes.</p>
        </div>
        {canSell && (
          <button className="btn btn--primary btn--large" onClick={() => setCreating(true)}>
            Nova venda
          </button>
        )}
      </header>
      {error && <Notice tone="error">{error}</Notice>}
      {notice && <Notice tone="success">{notice}</Notice>}
      {loading && !data ? (
        <p className="muted">Carregando…</p>
      ) : (
        <DataGrid
          rows={data ?? []}
          columns={columns}
          rowKey={(s) => s.id}
          onRowOpen={(s) => setOpenSale(s.id)}
          rowClass={(s) => (s.status === "cancelada" ? "is-canceled" : undefined)}
          exportName="Vendas"
          emptyText="Nenhuma venda no período."
          initialSort={{ key: "sold_at", dir: "desc" }}
          toolbar={
            <>
              <PeriodPicker value={period} onChange={setPeriod} />
              <select aria-label="Situação" value={status} onChange={(e) => setStatus(e.target.value)}>
                <option value="">Todas</option>
                <option value="concluida">Concluídas</option>
                <option value="cancelada">Canceladas</option>
              </select>
            </>
          }
        />
      )}

      <Dialog wide open={creating} title="Nova venda" onClose={() => setCreating(false)}>
        {creating && (
          <NewSaleForm
            onDone={async (s) => {
              setCreating(false);
              setNotice(`Venda nº ${s.number} registrada: ${money(s.total)} (${PAYMENT_LABEL[s.payment_method]}).`);
              await reload();
            }}
          />
        )}
      </Dialog>
      <Dialog wide open={!!openSale} title="Detalhes da venda" onClose={() => setOpenSale(null)}>
        {openSale && (
          <SaleDetail
            id={openSale}
            onCanceled={async (s) => {
              setNotice(`Venda nº ${s.number} cancelada; estoque devolvido.`);
              await reload();
            }}
          />
        )}
      </Dialog>
    </div>
  );
}

// ------------------------------------------------------------------ nova venda

interface Line {
  product: Product;
  qty: string;
  unitPrice: string;
  discount: string;
}

/** Centavos de uma linha, igual ao núcleo: qtd(milésimos) × preço(centavos) / 1000, meio para cima. */
function lineCents(l: Line): number {
  const q = Math.round(toNum(parseDecimal(l.qty) ?? 0) * 1000);
  const p = Math.round(toNum(parseDecimal(l.unitPrice) ?? 0) * 100);
  const d = Math.round(toNum(parseDecimal(l.discount) ?? 0) * 100);
  return Math.floor((q * p + 500) / 1000) - d;
}

function NewSaleForm({ onDone }: { onDone: (s: Sale) => Promise<void> }) {
  const { me, can } = useAuth();
  const canSetPrice = can("sales", "manage");
  const { data: products } = useAsync(productsApi.list);
  const { data: customers, setData: setCustomers } = useAsync(() =>
    can("customers", "read") ? customersApi.list() : Promise.resolve([] as Customer[]),
  );
  const { data: sellers } = useAsync(salesApi.sellers);
  const [lines, setLines] = useState<Line[]>([]);
  const [search, setSearch] = useState("");
  const [highlight, setHighlight] = useState(0);
  const [customerId, setCustomerId] = useState("");
  const [sellerId, setSellerId] = useState(me?.user.id ?? "");
  const [payment, setPayment] = useState<PaymentMethod>("dinheiro");
  const [discount, setDiscount] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [newCustomer, setNewCustomer] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);

  const active = useMemo(() => (products ?? []).filter((p) => p.active), [products]);
  const matches = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return [];
    return active
      .filter((p) => p.code.toLowerCase().includes(q) || p.barcode === q || p.name.toLowerCase().includes(q))
      .slice(0, 8);
  }, [active, search]);

  function add(p: Product) {
    setLines((ls) => {
      const i = ls.findIndex((l) => l.product.id === p.id);
      if (i >= 0) {
        const next = ls.slice();
        const q = toNum(parseDecimal(next[i].qty) ?? 0) + 1;
        next[i] = { ...next[i], qty: decimalInput(String(q)) };
        return next;
      }
      return [...ls, { product: p, qty: "1", unitPrice: decimalInput(p.price), discount: "" }];
    });
    setSearch("");
    setHighlight(0);
    searchRef.current?.focus();
  }

  function onSearchKey(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlight((h) => Math.min(h + 1, matches.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlight((h) => Math.max(h - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const q = search.trim().toLowerCase();
      // leitor de código de barras: código exato entra direto
      const exact = active.find((p) => p.barcode === search.trim() || p.code.toLowerCase() === q);
      const pick = exact ?? matches[highlight];
      if (pick) add(pick);
    } else if (e.key === "Escape") {
      setSearch("");
    }
  }

  const subtotal = lines.reduce((acc, l) => acc + lineCents(l), 0);
  const discountCents = Math.round(toNum(parseDecimal(discount) ?? 0) * 100);
  const total = subtotal - discountCents;

  async function submit(e?: FormEvent) {
    e?.preventDefault();
    if (!lines.length) return setError("Adicione pelo menos um produto.");
    const items = [];
    for (const l of lines) {
      const q = parseDecimal(l.qty);
      const price = parseDecimal(l.unitPrice);
      const d = l.discount.trim() ? parseDecimal(l.discount) : null;
      if (!q || toNum(q) <= 0 || price == null || (l.discount.trim() && d == null)) {
        return setError(`Confira a quantidade, o preço e o desconto de "${l.product.name}".`);
      }
      items.push({
        product_id: l.product.id,
        qty: q,
        unit_price: canSetPrice ? price : null,
        discount: d,
      });
    }
    if (discount.trim() && parseDecimal(discount) == null) return setError("Desconto da venda inválido.");
    setBusy(true);
    setError(null);
    try {
      const sale = await salesApi.create({
        customer_id: customerId || null,
        seller_id: sellerId || null,
        payment_method: payment,
        discount: discount.trim() ? parseDecimal(discount) : null,
        notes: notes.trim() || null,
        items,
      });
      await onDone(sale);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Não foi possível registrar a venda.");
    } finally {
      setBusy(false);
    }
  }

  // F9 finaliza (atalho de caixa)
  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === "F9") {
        e.preventDefault();
        void submit();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const setLine = (i: number, patch: Partial<Line>) => setLines((ls) => ls.map((l, j) => (j === i ? { ...l, ...patch } : l)));

  return (
    <form className="sale-form" onSubmit={submit}>
      <div className="sale-main">
        <div className="sale-search">
          <input
            ref={searchRef}
            autoFocus
            type="search"
            placeholder="Buscar produto por nome, código ou código de barras…"
            aria-label="Buscar produto"
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setHighlight(0);
            }}
            onKeyDown={onSearchKey}
          />
          {matches.length > 0 && (
            <ul className="sale-suggest" role="listbox">
              {matches.map((p, i) => (
                <li key={p.id} role="option" aria-selected={i === highlight}>
                  <button type="button" onMouseDown={(e) => e.preventDefault()} onClick={() => add(p)}>
                    <span>
                      <span className="muted">{p.code}</span> {p.name}
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

        <div className="table-wrap">
          <table className="table sale-items">
            <thead>
              <tr>
                <th>Produto</th>
                <th className="num">Qtd.</th>
                <th className="num">Preço un.</th>
                <th className="num">Desconto</th>
                <th className="num">Total</th>
                <th aria-label="Remover" />
              </tr>
            </thead>
            <tbody>
              {lines.map((l, i) => {
                const overStock = l.product.track_stock && toNum(parseDecimal(l.qty) ?? 0) > toNum(l.product.stock);
                return (
                  <tr key={l.product.id}>
                    <td>
                      {l.product.name}
                      {overStock && <div className="field-error">Estoque: {qty(l.product.stock)} {l.product.unit}</div>}
                    </td>
                    <td className="num">
                      <input aria-label={`Quantidade de ${l.product.name}`} inputMode="decimal" className="cell-input"
                        value={l.qty} onChange={(e) => setLine(i, { qty: e.target.value })} />
                    </td>
                    <td className="num">
                      {canSetPrice ? (
                        <input aria-label={`Preço de ${l.product.name}`} inputMode="decimal" className="cell-input"
                          value={l.unitPrice} onChange={(e) => setLine(i, { unitPrice: e.target.value })} />
                      ) : (
                        money(l.product.price)
                      )}
                    </td>
                    <td className="num">
                      <input aria-label={`Desconto em ${l.product.name}`} inputMode="decimal" className="cell-input" placeholder="0,00"
                        value={l.discount} onChange={(e) => setLine(i, { discount: e.target.value })} />
                    </td>
                    <td className="num">{money(lineCents(l) / 100)}</td>
                    <td>
                      <button type="button" className="btn btn--ghost" aria-label={`Remover ${l.product.name}`}
                        onClick={() => setLines((ls) => ls.filter((_, j) => j !== i))}>✕</button>
                    </td>
                  </tr>
                );
              })}
              {!lines.length && (
                <tr>
                  <td colSpan={6} className="grid-empty">Busque e adicione produtos. Leitor de código de barras funciona aqui.</td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      <aside className="sale-side">
        {error && <Notice tone="error">{error}</Notice>}
        <div className="field">
          <label htmlFor="ns-customer">Cliente</label>
          <div className="row">
            <select id="ns-customer" value={customerId} onChange={(e) => setCustomerId(e.target.value)}>
              <option value="">Consumidor (sem cadastro)</option>
              {(customers ?? []).filter((c) => c.active).map((c) => (
                <option key={c.id} value={c.id}>{c.name}{c.city ? ` · ${c.city}` : ""}</option>
              ))}
            </select>
            {can("customers", "write") && (
              <button type="button" className="btn btn--ghost" onClick={() => setNewCustomer(true)} aria-label="Novo cliente">+</button>
            )}
          </div>
        </div>
        <div className="field">
          <label htmlFor="ns-seller">Vendedor</label>
          <select id="ns-seller" value={sellerId} onChange={(e) => setSellerId(e.target.value)}>
            {(sellers ?? []).map((s) => (
              <option key={s.id} value={s.id}>{s.name}</option>
            ))}
          </select>
        </div>
        <div className="field">
          <span className="field-label">Pagamento</span>
          <div className="pay-grid" role="radiogroup" aria-label="Pagamento">
            {PAYMENTS.map((p) => (
              <label key={p} className={payment === p ? "is-on" : undefined}>
                <input type="radio" name="payment" checked={payment === p} onChange={() => setPayment(p)} />
                {PAYMENT_LABEL[p]}
              </label>
            ))}
          </div>
        </div>
        <Field label="Desconto na venda (R$)" inputMode="decimal" placeholder="0,00" value={discount} onChange={(e) => setDiscount(e.target.value)} />
        <Field label="Observação" value={notes} onChange={(e) => setNotes(e.target.value)} />
        <dl className="sale-totals">
          <div><dt>Subtotal</dt><dd>{money(subtotal / 100)}</dd></div>
          {discountCents > 0 && <div><dt>Desconto</dt><dd>− {money(discountCents / 100)}</dd></div>}
          <div className="sale-total"><dt>Total</dt><dd>{money(total / 100)}</dd></div>
        </dl>
        <button className="btn btn--primary btn--block btn--large" disabled={busy || !lines.length}>
          {busy ? "Registrando…" : "Finalizar venda (F9)"}
        </button>
      </aside>

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

function SaleDetail({ id, onCanceled }: { id: string; onCanceled: (s: Sale) => Promise<void> }) {
  const { can } = useAuth();
  const { data: sale, error, setData } = useAsync(() => salesApi.get(id), [id]);
  const [reason, setReason] = useState("");
  const [canceling, setCanceling] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  if (error) return <Notice tone="error">{error}</Notice>;
  if (!sale) return <p className="muted">Carregando…</p>;

  const itemColumns: GridColumn<SaleItem>[] = [
    { key: "name", label: "Produto", get: (i) => i.product_name },
    { key: "qty", label: "Qtd.", kind: "number", get: (i) => i.qty },
    { key: "unit_price", label: "Preço un.", kind: "money", get: (i) => i.unit_price },
    { key: "discount", label: "Desconto", kind: "money", get: (i) => i.discount, total: "sum" },
    { key: "total", label: "Total", kind: "money", get: (i) => i.total, total: "sum" },
  ];

  async function cancel(e: FormEvent) {
    e.preventDefault();
    setActionError(null);
    try {
      const s = await salesApi.cancel(id, reason);
      setData(s);
      setCanceling(false);
      await onCanceled(s);
    } catch (err) {
      setActionError(err instanceof ApiError ? err.message : "Não foi possível cancelar.");
    }
  }

  return (
    <div className="stack">
      <div className="kv-grid">
        <div><span>Venda</span><strong>nº {sale.number}</strong></div>
        <div><span>Data</span><strong>{dateTime(sale.sold_at)}</strong></div>
        <div><span>Cliente</span><strong>{sale.customer_name ?? "Consumidor"}</strong></div>
        <div><span>Vendedor</span><strong>{sale.seller_name}</strong></div>
        <div><span>Pagamento</span><strong>{PAYMENT_LABEL[sale.payment_method]}</strong></div>
        <div><span>Total</span><strong>{money(sale.total)}</strong></div>
        {sale.profit != null && <div><span>Lucro</span><strong>{money(sale.profit)}</strong></div>}
        <div>
          <span>Situação</span>
          <strong>{sale.status === "cancelada" ? <span className="tag tag--danger">Cancelada</span> : <span className="tag tag--ok">Concluída</span>}</strong>
        </div>
      </div>
      {sale.status === "cancelada" && (
        <Notice tone="error">Cancelada em {dateTime(sale.canceled_at)}: {sale.cancel_reason}</Notice>
      )}
      {sale.notes && <p className="muted">Obs.: {sale.notes}</p>}
      <DataGrid rows={sale.items ?? []} columns={itemColumns} rowKey={(i) => i.id} maxHeight="40vh" exportName={`Venda-${sale.number}`} />
      {toNum(sale.discount) > 0 && <p className="muted">Desconto na venda: {money(sale.discount)}</p>}

      {actionError && <Notice tone="error">{actionError}</Notice>}
      {sale.status === "concluida" && can("sales", "manage") && (
        canceling ? (
          <form className="row" onSubmit={cancel}>
            <input aria-label="Motivo do cancelamento" placeholder="Motivo do cancelamento" required minLength={3} autoFocus
              value={reason} onChange={(e) => setReason(e.target.value)} />
            <button className="btn btn--primary">Confirmar cancelamento</button>
            <button type="button" className="btn btn--ghost" onClick={() => setCanceling(false)}>Voltar</button>
          </form>
        ) : (
          <div className="row row--end">
            <button type="button" className="btn btn--ghost" onClick={() => setCanceling(true)}>Cancelar venda</button>
          </div>
        )
      )}
    </div>
  );
}
