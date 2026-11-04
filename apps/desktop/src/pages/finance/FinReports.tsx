import { Printer } from "lucide-react";
import { useMemo, useState } from "react";

import { ColumnChart, StatTile } from "../../components/charts";
import { DataGrid, type GridColumn } from "../../components/DataGrid";
import { Notice } from "../../components/Notice";
import { PeriodPicker } from "../../components/PeriodPicker";
import { useAsync } from "../../hooks/useAsync";
import { financeApi } from "../../services/endpoints";
import { money, moneyShort, toNum, tzOffset } from "../../services/format";
import { presetPeriod, type Period } from "../../services/period";
import type { CashFlow, Dre } from "../../services/types";

const MONTHS = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];
const monthLabel = (k: string) => `${MONTHS[Number(k.slice(5, 7)) - 1]}/${k.slice(2, 4)}`;
const label = (k: string) => (k.length === 7 ? monthLabel(k) : `${k.slice(8, 10)}/${k.slice(5, 7)}`);

export function CashFlowReport() {
  const [period, setPeriod] = useState<Period>(() => {
    const now = new Date();
    const p = presetPeriod("month");
    const end = new Date(now.getFullYear(), now.getMonth() + 1, 0); // até o fim do mês: inclui o previsto
    return { ...p, preset: "custom", to: `${end.getFullYear()}-${String(end.getMonth() + 1).padStart(2, "0")}-${String(end.getDate()).padStart(2, "0")}` };
  });
  const [group, setGroup] = useState<"day" | "month">("day");
  const { data, error, loading } = useAsync(() => financeApi.cashFlow({ from: period.from, to: period.to, group }), [period.from, period.to, group]);
  type Row = CashFlow["rows"][number];
  const columns: GridColumn<Row>[] = [
    { key: "period", label: "Período", get: (r) => label(r.period) },
    { key: "in", label: "Entradas", kind: "money", get: (r) => r.realized_in, total: "sum" },
    { key: "out", label: "Saídas", kind: "money", get: (r) => r.realized_out, total: "sum" },
    { key: "bal", label: "Saldo", kind: "money", get: (r) => r.balance },
    { key: "pin", label: "A receber (previsto)", kind: "money", get: (r) => r.projected_in, total: "sum" },
    { key: "pout", label: "A pagar (previsto)", kind: "money", get: (r) => r.projected_out, total: "sum" },
    { key: "pbal", label: "Saldo previsto", kind: "money", get: (r) => r.projected_balance },
  ];
  return (
    <div className={loading ? "report is-loading" : "report"}>
      <div className="filters no-print">
        <PeriodPicker value={period} onChange={setPeriod} />
        <select aria-label="Agrupar" value={group} onChange={(e) => setGroup(e.target.value as "day" | "month")}>
          <option value="day">Por dia</option>
          <option value="month">Por mês</option>
        </select>
      </div>
      {error && <Notice tone="error">{error}</Notice>}
      {data && (
        <>
          <div className="stats">
            <StatTile label="Saldo no início" count={toNum(data.opening_balance)} format={money} />
            <StatTile label="Saldo no fim (realizado)" count={toNum(data.closing_balance)} format={money} />
            <StatTile label="Saldo previsto" count={toNum(data.projected_closing)} format={money} hint="incluindo títulos em aberto" />
          </div>
          <section className="panel">
            <h2>Entradas × saídas realizadas</h2>
            <ColumnChart
              ariaLabel="Entradas e saídas por período"
              legend={["Entradas", "Saídas"]}
              data={data.rows.map((r) => ({ key: r.period, label: label(r.period), value: toNum(r.realized_in), previous: toNum(r.realized_out) }))}
              format={money}
              axisFormat={moneyShort}
            />
          </section>
          <section className="panel">
            <h2>Fluxo de caixa</h2>
            <DataGrid rows={data.rows} columns={columns} rowKey={(r) => r.period} exportName="Fluxo de caixa" maxHeight="55vh" />
          </section>
        </>
      )}
    </div>
  );
}

function monthsAgo(n: number) {
  const d = new Date();
  d.setDate(1);
  d.setMonth(d.getMonth() - n);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
}

export function DreReport() {
  const [from, setFrom] = useState(monthsAgo(2));
  const [to, setTo] = useState(monthsAgo(0));
  const { data, error, loading } = useAsync(() => financeApi.dre({ from_month: from, to_month: to, tz_offset_minutes: tzOffset() }), [from, to]);
  type Line = Dre["lines"][number];
  const columns: GridColumn<Line>[] = useMemo(
    () => [
      { key: "label", label: "Demonstrativo", get: (l) => l.label, width: 280, render: (l) => <span className={l.kind === "subtotal" ? "dre-subtotal" : l.kind === "info" ? "muted" : undefined}>{l.label}</span> },
      ...(data?.months ?? []).map(
        (m, i): GridColumn<Line> => ({
          key: m,
          label: monthLabel(m),
          kind: "money",
          get: (l) => l.values[i],
          render: (l) => <span className={`${l.kind === "subtotal" ? "dre-subtotal" : ""} ${toNum(l.values[i]) < 0 ? "neg" : ""}`}>{money(l.values[i])}</span>,
        }),
      ),
      { key: "total", label: "Total", kind: "money", get: (l) => l.total, render: (l) => <strong className={toNum(l.total) < 0 ? "neg" : undefined}>{money(l.total)}</strong> },
    ],
    [data],
  );
  const result = data?.lines.find((l) => l.key === "resultado");
  const revenue = data?.lines.find((l) => l.key === "receita_bruta");
  return (
    <div className={loading ? "report is-loading" : "report"}>
      <div className="filters no-print">
        <label className="row">De <input type="month" value={from} max={to} onChange={(e) => e.target.value && setFrom(e.target.value)} style={{ width: "auto" }} /></label>
        <label className="row">até <input type="month" value={to} min={from} onChange={(e) => e.target.value && setTo(e.target.value)} style={{ width: "auto" }} /></label>
        <button type="button" className="btn btn--ghost" onClick={() => window.print()}><Printer size={16} aria-hidden /> Imprimir / PDF</button>
      </div>
      {error && <Notice tone="error">{error}</Notice>}
      {data && (
        <>
          <div className="stats">
            <StatTile label="Receita bruta" count={toNum(revenue?.total)} format={money} />
            <StatTile label="Resultado líquido" count={toNum(result?.total)} format={money} tone={toNum(result?.total) < 0 ? "warn" : undefined}
              hint={toNum(revenue?.total) ? `margem ${((toNum(result?.total) / toNum(revenue?.total)) * 100).toFixed(1).replace(".", ",")}%` : undefined} />
          </div>
          <section className="panel">
            <h2>DRE gerencial</h2>
            <p className="muted small">
              Por competência: vendas pela data da venda (com o custo guardado em cada venda) e títulos pela data de emissão.
              Lançamentos avulsos entram pela categoria. Valores negativos são custos e despesas.
            </p>
            <DataGrid rows={data.lines} columns={columns} rowKey={(l) => l.key} exportName="DRE" rowClass={(l) => (l.kind === "subtotal" ? "is-subtotal" : undefined)} />
          </section>
          <p className="muted small no-print">{/* PLANO: DRE é relatório avançado (Platinum+) */}Relatório avançado: no futuro faz parte do plano Platinum; hoje está liberado. Gráfico mensal em Relatórios → Vendas.</p>
        </>
      )}
    </div>
  );
}
