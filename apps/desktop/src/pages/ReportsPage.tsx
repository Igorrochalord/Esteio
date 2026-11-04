import { useMemo, useState } from "react";

import { BarList, BrazilTileMap, ColumnChart, StatTile } from "../components/charts";
import { DataGrid, type GridColumn } from "../components/DataGrid";
import { Notice } from "../components/Notice";
import { PeriodPicker } from "../components/PeriodPicker";
import { useAsync } from "../hooks/useAsync";
import { reportsApi } from "../services/endpoints";
import { date, int, money, moneyShort, pct, qty, toNum, tzOffset } from "../services/format";
import { PAYMENT_LABEL } from "../services/labels";
import { periodBounds, periodDays, presetPeriod, type Period } from "../services/period";
import type { PeriodRow, ProductRank, ReportQuery, SellerRow, StockRow } from "../services/types";

type Tab = "sales" | "products" | "stock" | "customers" | "sellers";
type Group = "auto" | "day" | "week" | "month";

const TABS: { key: Tab; label: string }[] = [
  { key: "sales", label: "Vendas" },
  { key: "products", label: "Produtos" },
  { key: "stock", label: "Estoque" },
  { key: "customers", label: "Clientes e mapa" },
  { key: "sellers", label: "Vendedores" },
];

const MONTHS = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];

/** Rótulo curto do período: 01/10 · sem. 40 · out/26 */
function periodLabel(key: string): string {
  if (/^\d{4}-\d{2}-\d{2}$/.test(key)) return `${key.slice(8, 10)}/${key.slice(5, 7)}`;
  if (/^\d{4}-W\d{2}$/.test(key)) return `sem. ${Number(key.slice(6))}`;
  if (/^\d{4}-\d{2}$/.test(key)) return `${MONTHS[Number(key.slice(5, 7)) - 1]}/${key.slice(2, 4)}`;
  return key;
}

export function ReportsPage() {
  const [period, setPeriod] = useState<Period>(() => presetPeriod("30d"));
  const [group, setGroup] = useState<Group>("auto");
  const [tab, setTab] = useState<Tab>("sales");
  const days = periodDays(period);
  const effective = group === "auto" ? (days <= 62 ? "day" : days <= 200 ? "week" : "month") : group;
  const query: ReportQuery = useMemo(
    () => ({ ...periodBounds(period), tz_offset_minutes: tzOffset(), group: effective }),
    [period, effective],
  );

  return (
    <div className="page page--wide">
      <header className="page-header">
        <div>
          <h1>Relatórios</h1>
          <p className="muted">
            Vendas canceladas ficam fora dos totais. Toda tabela pode ser exportada para o Excel.
          </p>
        </div>
      </header>

      <div className="filters">
        <PeriodPicker value={period} onChange={setPeriod} />
        {tab === "sales" && (
          <select aria-label="Agrupar por" value={group} onChange={(e) => setGroup(e.target.value as Group)}>
            <option value="auto">Agrupar: automático</option>
            <option value="day">Por dia</option>
            <option value="week">Por semana</option>
            <option value="month">Por mês</option>
          </select>
        )}
        <span className="muted small">
          {date(`${period.from}T12:00:00`)} a {date(`${period.to}T12:00:00`)} · {days} dia(s)
        </span>
      </div>

      <nav className="tabs" role="tablist" aria-label="Relatórios">
        {TABS.map((t) => (
          <button
            key={t.key}
            role="tab"
            type="button"
            aria-selected={tab === t.key}
            className={tab === t.key ? "tab is-on" : "tab"}
            onClick={() => setTab(t.key)}
          >
            {t.label}
          </button>
        ))}
      </nav>

      <div role="tabpanel">
        {tab === "sales" && <SalesTab query={query} />}
        {tab === "products" && <ProductsTab query={query} />}
        {tab === "stock" && <StockTab query={query} />}
        {tab === "customers" && <CustomersTab query={query} />}
        {tab === "sellers" && <SellersTab query={query} />}
      </div>
    </div>
  );
}

const deps = (q: ReportQuery) => [q.from, q.to, q.group, q.tz_offset_minutes];

function Loading({ loading, error, children }: { loading: boolean; error: string | null; children: React.ReactNode }) {
  return (
    <div className={loading ? "report is-loading" : "report"}>
      {error && <Notice tone="error">{error}</Notice>}
      {children}
    </div>
  );
}

// ------------------------------------------------------------------ vendas

function SalesTab({ query }: { query: ReportQuery }) {
  const { data, error, loading } = useAsync(() => reportsApi.sales(query), deps(query));
  if (!data) return <Loading loading error={error}><p className="muted">Carregando…</p></Loading>;
  const t = data.totals;
  const unit = query.group === "month" ? "mês" : query.group === "week" ? "semana" : "dia";
  const columns: GridColumn<PeriodRow>[] = [
    { key: "period", label: "Período", get: (r) => periodLabel(r.period) },
    { key: "count", label: "Vendas", kind: "number", get: (r) => r.count, total: "sum" },
    { key: "total", label: "Faturamento", kind: "money", get: (r) => r.total, total: "sum" },
    { key: "cost", label: "Custo", kind: "money", get: (r) => r.cost, total: "sum" },
    { key: "profit", label: "Lucro", kind: "money", get: (r) => r.profit, total: "sum" },
    {
      key: "margin",
      label: "Margem",
      kind: "percent",
      get: (r) => (toNum(r.total) ? ((toNum(r.profit) / toNum(r.total)) * 100).toFixed(1) : null),
    },
    {
      key: "ticket",
      label: "Ticket médio",
      kind: "money",
      get: (r) => (r.count ? (toNum(r.total) / r.count).toFixed(2) : null),
    },
  ];

  return (
    <Loading loading={loading} error={error}>
      <div className="stats">
        <StatTile label="Faturamento" value={money(t.total)} hint={toNum(t.discount) > 0 ? `${money(t.discount)} em descontos` : undefined} />
        <StatTile label="Vendas" value={int(t.count)} hint={`${qty(t.items_qty)} itens`} />
        <StatTile label="Ticket médio" value={money(t.avg_ticket)} />
        <StatTile label="Lucro bruto" value={money(t.profit)} hint={`custo ${money(t.cost)}`} />
        <StatTile label="Margem" value={pct(t.margin_pct)} />
        <StatTile label="Canceladas" value={int(t.canceled_count)} hint={t.canceled_count ? money(t.canceled_total) : undefined} />
      </div>

      <section className="panel">
        <h2>Faturamento por {unit}</h2>
        <ColumnChart
          ariaLabel={`Faturamento por ${unit}`}
          data={data.series.map((r) => ({
            key: r.period,
            label: periodLabel(r.period),
            value: toNum(r.total),
            detail: (
              <>
                <span>{int(r.count)} venda(s)</span>
                <span>lucro {money(r.profit)}</span>
              </>
            ),
          }))}
          format={money}
          axisFormat={moneyShort}
        />
      </section>

      <div className="two-col">
        <section className="panel">
          <h2>Por forma de pagamento</h2>
          <BarList
            ariaLabel="Faturamento por forma de pagamento"
            data={data.by_payment.map((p) => ({
              key: p.method,
              label: PAYMENT_LABEL[p.method] ?? p.method,
              value: toNum(p.total),
              note: `${pct(p.share_pct)} · ${int(p.count)} venda(s)`,
            }))}
            format={money}
          />
        </section>
        <section className="panel">
          <h2>Por hora do dia</h2>
          <ColumnChart
            ariaLabel="Faturamento por hora do dia"
            height={180}
            data={data.by_hour.map((h) => ({
              key: h.period,
              label: `${Number(h.period)}h`,
              value: toNum(h.total),
              detail: <span>{int(h.count)} venda(s)</span>,
            }))}
            format={money}
            axisFormat={moneyShort}
          />
        </section>
      </div>

      <section className="panel">
        <h2>Tabela</h2>
        <DataGrid rows={data.series} columns={columns} rowKey={(r) => r.period} exportName="Vendas por periodo" maxHeight="50vh" />
      </section>
    </Loading>
  );
}

// ------------------------------------------------------------------ produtos

function ProductsTab({ query }: { query: ReportQuery }) {
  const { data, error, loading } = useAsync(() => reportsApi.products(query), deps(query));
  if (!data) return <Loading loading error={error}><p className="muted">Carregando…</p></Loading>;
  const abcCount = (c: string) => data.products.filter((p) => p.abc === c).length;
  const columns: GridColumn<ProductRank>[] = [
    { key: "code", label: "Código", get: (p) => p.code },
    { key: "name", label: "Produto", get: (p) => p.name, width: 200 },
    { key: "category", label: "Categoria", get: (p) => p.category },
    { key: "qty", label: "Qtd. vendida", kind: "number", get: (p) => p.qty, total: "sum" },
    { key: "revenue", label: "Faturamento", kind: "money", get: (p) => p.revenue, total: "sum" },
    { key: "cost", label: "Custo", kind: "money", get: (p) => p.cost, total: "sum" },
    { key: "profit", label: "Lucro", kind: "money", get: (p) => p.profit, total: "sum" },
    { key: "margin", label: "Margem", kind: "percent", get: (p) => p.margin_pct },
    { key: "share", label: "Participação", kind: "percent", get: (p) => p.share_pct },
    { key: "cumulative", label: "Acumulado", kind: "percent", get: (p) => p.cumulative_pct },
    {
      key: "abc",
      label: "Curva",
      get: (p) => p.abc,
      render: (p) => <span className={p.abc === "A" ? "tag tag--ok" : p.abc === "B" ? "tag" : "tag tag--warn"}>{p.abc}</span>,
    },
  ];

  return (
    <Loading loading={loading} error={error}>
      <div className="stats">
        <StatTile label="Produtos vendidos" value={int(data.products.length)} />
        <StatTile label="Curva A" value={int(abcCount("A"))} hint="geram 80% do faturamento" />
        <StatTile label="Curva B" value={int(abcCount("B"))} hint="os próximos 15%" />
        <StatTile label="Curva C" value={int(abcCount("C"))} hint="os últimos 5%" />
      </div>
      <div className="two-col">
        <section className="panel">
          <h2>Mais vendidos (faturamento)</h2>
          <BarList
            ariaLabel="Dez produtos com maior faturamento"
            data={data.products.slice(0, 10).map((p) => ({
              key: p.product_id,
              label: p.name,
              value: toNum(p.revenue),
              note: `${qty(p.qty)} un. · curva ${p.abc}`,
            }))}
            format={money}
          />
        </section>
        <section className="panel">
          <h2>Lucro por categoria</h2>
          <BarList
            ariaLabel="Lucro por categoria"
            data={data.categories
              .slice()
              .sort((a, b) => toNum(b.profit) - toNum(a.profit))
              .map((c) => ({ key: c.category, label: c.category, value: toNum(c.profit), note: `margem ${pct(c.margin_pct)}` }))}
            format={money}
          />
        </section>
      </div>
      <section className="panel">
        <h2>Todos os produtos vendidos</h2>
        <p className="muted small">
          Faturamento por item, antes do desconto dado no total da venda. Curva ABC pelo acumulado do faturamento.
        </p>
        <DataGrid rows={data.products} columns={columns} rowKey={(p) => p.product_id} exportName="Produtos vendidos" maxHeight="55vh" />
      </section>
    </Loading>
  );
}

// ------------------------------------------------------------------ estoque

function StockTab({ query }: { query: ReportQuery }) {
  const { data, error, loading } = useAsync(() => reportsApi.stock(query), deps(query));
  const [onlyLow, setOnlyLow] = useState(false);
  if (!data) return <Loading loading error={error}><p className="muted">Carregando…</p></Loading>;
  const rows = onlyLow ? data.rows.filter((r) => r.below_min) : data.rows;
  const columns: GridColumn<StockRow>[] = [
    { key: "code", label: "Código", get: (r) => r.code },
    { key: "name", label: "Produto", get: (r) => r.name, width: 200 },
    { key: "category", label: "Categoria", get: (r) => r.category },
    { key: "stock", label: "Estoque", kind: "number", get: (r) => r.stock },
    { key: "unit", label: "Un.", get: (r) => r.unit },
    { key: "min", label: "Mínimo", kind: "number", get: (r) => r.min_stock },
    { key: "sold", label: "Vendido no período", kind: "number", get: (r) => r.sold_qty },
    { key: "cover", label: "Dura (dias)", kind: "number", get: (r) => r.days_of_cover },
    { key: "cost_value", label: "Valor (custo)", kind: "money", get: (r) => r.cost_value, total: "sum" },
    { key: "price_value", label: "Valor (venda)", kind: "money", get: (r) => r.price_value, total: "sum" },
    {
      key: "status",
      label: "Situação",
      get: (r) => (toNum(r.stock) <= 0 ? "Sem estoque" : r.below_min ? "Abaixo do mínimo" : "OK"),
      render: (r) =>
        toNum(r.stock) <= 0 ? (
          <span className="tag tag--danger">⨯ Sem estoque</span>
        ) : r.below_min ? (
          <span className="tag tag--warn">⚠ Abaixo do mínimo</span>
        ) : (
          <span className="tag tag--ok">✓ OK</span>
        ),
    },
  ];
  return (
    <Loading loading={loading} error={error}>
      <div className="stats">
        <StatTile label="Valor em estoque (custo)" value={money(data.total_cost_value)} />
        <StatTile label="Valor em estoque (venda)" value={money(data.total_price_value)} />
        <StatTile label="Abaixo do mínimo" value={int(data.below_min_count)} tone={data.below_min_count ? "warn" : undefined} />
        <StatTile label="Sem estoque" value={int(data.out_of_stock_count)} tone={data.out_of_stock_count ? "warn" : undefined} />
      </div>
      <section className="panel">
        <h2>Posição de estoque</h2>
        <p className="muted small">"Dura (dias)" estima quantos dias o estoque atual dura no ritmo de venda do período.</p>
        <DataGrid
          rows={rows}
          columns={columns}
          rowKey={(r) => r.product_id}
          exportName="Estoque"
          maxHeight="60vh"
          rowClass={(r) => (r.below_min ? "is-warning" : undefined)}
          toolbar={
            <label className="check">
              <input type="checkbox" checked={onlyLow} onChange={(e) => setOnlyLow(e.target.checked)} />
              Só abaixo do mínimo
            </label>
          }
        />
      </section>
    </Loading>
  );
}

// ------------------------------------------------------------------ clientes

function CustomersTab({ query }: { query: ReportQuery }) {
  const { data, error, loading } = useAsync(() => reportsApi.customers(query), deps(query));
  if (!data) return <Loading loading error={error}><p className="muted">Carregando…</p></Loading>;
  type Top = (typeof data.top_customers)[number];
  const columns: GridColumn<Top>[] = [
    { key: "name", label: "Cliente", get: (c) => c.name, width: 200 },
    { key: "city", label: "Cidade", get: (c) => c.city },
    { key: "state", label: "UF", get: (c) => c.state },
    { key: "count", label: "Compras", kind: "number", get: (c) => c.count, total: "sum" },
    { key: "total", label: "Total", kind: "money", get: (c) => c.total, total: "sum" },
    { key: "avg", label: "Ticket médio", kind: "money", get: (c) => c.avg_ticket },
    { key: "last", label: "Última compra", kind: "date", get: (c) => c.last_purchase },
  ];
  type StateRow = (typeof data.by_state)[number];
  const stateColumns: GridColumn<StateRow>[] = [
    { key: "state", label: "UF", get: (s) => s.state },
    { key: "customers", label: "Clientes", kind: "number", get: (s) => s.customers, total: "sum" },
    { key: "count", label: "Vendas", kind: "number", get: (s) => s.count, total: "sum" },
    { key: "total", label: "Total", kind: "money", get: (s) => s.total, total: "sum" },
  ];
  return (
    <Loading loading={loading} error={error}>
      <div className="two-col two-col--map">
        <section className="panel">
          <h2>Vendas por estado</h2>
          <p className="muted small">Cada quadrado é uma UF, na posição aproximada do mapa. Quanto mais escuro, mais vendas.</p>
          <BrazilTileMap
            ariaLabel="Mapa de vendas por estado"
            data={data.by_state.map((s) => ({
              state: s.state,
              value: toNum(s.total),
              detail: (
                <>
                  <span>{int(s.customers)} cliente(s)</span>
                  <span>{int(s.count)} venda(s)</span>
                </>
              ),
            }))}
            format={money}
          />
        </section>
        <section className="panel">
          <h2>Cidades que mais compram</h2>
          <BarList
            ariaLabel="Cidades por valor comprado"
            data={data.by_city.slice(0, 12).map((c) => ({
              key: `${c.city}-${c.state}`,
              label: c.state ? `${c.city} · ${c.state}` : c.city,
              value: toNum(c.total),
              note: `${int(c.customers)} cliente(s)`,
            }))}
            format={money}
          />
        </section>
      </div>
      {(data.anonymous_count > 0 || data.without_state > 0) && (
        <p className="muted small">
          {data.anonymous_count > 0 && <>Vendas sem cliente identificado: {int(data.anonymous_count)} ({money(data.anonymous_total)}). </>}
          {data.without_state > 0 && <>{int(data.without_state)} cliente(s) com compra não têm UF cadastrada e ficam fora do mapa.</>}
        </p>
      )}
      <div className="two-col">
        <section className="panel">
          <h2>Por estado</h2>
          <DataGrid rows={data.by_state} columns={stateColumns} rowKey={(s) => s.state} exportName="Vendas por UF" maxHeight="40vh" />
        </section>
        <section className="panel">
          <h2>Melhores clientes</h2>
          <DataGrid rows={data.top_customers} columns={columns} rowKey={(c) => c.customer_id} exportName="Melhores clientes" maxHeight="40vh" />
        </section>
      </div>
    </Loading>
  );
}

// ------------------------------------------------------------------ vendedores

function SellersTab({ query }: { query: ReportQuery }) {
  const { data, error, loading } = useAsync(() => reportsApi.sellers(query), deps(query));
  if (!data) return <Loading loading error={error}><p className="muted">Carregando…</p></Loading>;
  const columns: GridColumn<SellerRow>[] = [
    { key: "name", label: "Vendedor", get: (s) => s.name, width: 180 },
    { key: "count", label: "Vendas", kind: "number", get: (s) => s.count, total: "sum" },
    { key: "total", label: "Faturamento", kind: "money", get: (s) => s.total, total: "sum" },
    { key: "share", label: "Participação", kind: "percent", get: (s) => s.share_pct },
    { key: "avg", label: "Ticket médio", kind: "money", get: (s) => s.avg_ticket },
    { key: "profit", label: "Lucro", kind: "money", get: (s) => s.profit, total: "sum" },
    { key: "margin", label: "Margem", kind: "percent", get: (s) => s.margin_pct },
    { key: "customers", label: "Clientes atendidos", kind: "number", get: (s) => s.customers_served },
    { key: "items", label: "Itens", kind: "number", get: (s) => s.items_qty, total: "sum" },
    { key: "canceled", label: "Canceladas", kind: "number", get: (s) => s.canceled_count, total: "sum" },
  ];
  const best = data[0];
  return (
    <Loading loading={loading} error={error}>
      <div className="stats">
        <StatTile label="Vendedores com venda" value={int(data.filter((s) => s.count > 0).length)} />
        {best && <StatTile label="Destaque do período" value={best.name} hint={`${money(best.total)} · ${pct(best.share_pct)}`} />}
      </div>
      <section className="panel">
        <h2>Faturamento por vendedor</h2>
        <BarList
          ariaLabel="Faturamento por vendedor"
          data={data.map((s) => ({
            key: s.seller_id,
            label: s.name,
            value: toNum(s.total),
            note: `${pct(s.share_pct)} · ticket ${money(s.avg_ticket)}`,
          }))}
          format={money}
        />
      </section>
      <section className="panel">
        <h2>Desempenho</h2>
        <DataGrid rows={data} columns={columns} rowKey={(s) => s.seller_id} exportName="Vendedores" maxHeight="50vh" />
      </section>
    </Loading>
  );
}
