import {
  AlertTriangle,
  BarChart3,
  Boxes,
  CalendarDays,
  CircleCheck,
  ShoppingCart,
  TrendingUp,
  UserPlus,
  UserRound,
  type LucideIcon,
} from "lucide-react";
import { Link, useNavigate } from "react-router";

import { BarList, ColumnChart, StatTile } from "../components/charts";
import { useAsync } from "../hooks/useAsync";
import { useAuth } from "../hooks/useAuth";
import { financeApi, productsApi, reportsApi, salesApi, usersApi } from "../services/endpoints";
import { int, isoDay, localDayStart, money, moneyShort, nextDay, pctChange, qty, toNum, tzOffset } from "../services/format";

function greeting(): string {
  const h = new Date().getHours();
  return h < 12 ? "Bom dia" : h < 18 ? "Boa tarde" : "Boa noite";
}

const todayLong = () =>
  new Intl.DateTimeFormat("pt-BR", { weekday: "long", day: "numeric", month: "long" }).format(new Date());

/** Limites [início, fim) em dias locais. */
const range = (from: Date, toInclusive: Date) => ({
  from: localDayStart(isoDay(from)),
  to: nextDay(isoDay(toInclusive)),
  tz_offset_minutes: tzOffset(),
});

function ranges() {
  const now = new Date();
  const y = now.getFullYear();
  const m = now.getMonth();
  const d = now.getDate();
  const day = (yy: number, mm: number, dd: number) => new Date(yy, mm, dd);
  // mês passado até o mesmo dia (limitado ao último dia daquele mês)
  const lastOfPrev = day(y, m, 0).getDate();
  return {
    today: range(day(y, m, d), day(y, m, d)),
    yesterday: range(day(y, m, d - 1), day(y, m, d - 1)),
    month: range(day(y, m, 1), day(y, m, d)),
    prevMonth: range(day(y, m - 1, 1), day(y, m - 1, Math.min(d, lastOfPrev))),
    last30: { ...range(day(y, m, d - 29), day(y, m, d)), group: "day" as const },
    prev30: { ...range(day(y, m, d - 59), day(y, m, d - 30)), group: "day" as const },
  };
}

function QuickAction({ to, icon: Icon, label, hint }: { to: string; icon: LucideIcon; label: string; hint: string }) {
  return (
    <Link to={to} className="quick-action">
      <span className="quick-icon">
        <Icon size={20} aria-hidden />
      </span>
      <span>
        <strong>{label}</strong>
        <span className="muted small">{hint}</span>
      </span>
    </Link>
  );
}

export function DashboardPage() {
  const { me, can } = useAuth();
  const canReports = can("reports");
  if (!me) return null;
  const firstName = me.user.name.split(" ")[0];

  return (
    <div className="page page--wide dashboard">
      <header className="page-header">
        <div>
          <p className="muted small capitalize">
            <CalendarDays size={14} aria-hidden /> {todayLong()}
          </p>
          <h1>
            {greeting()}, {firstName}
          </h1>
          <p className="muted">{me.company.name}</p>
        </div>
      </header>

      <div className="quick-actions">
        {can("sales", "write") && <QuickAction to="/vendas?nova=1" icon={ShoppingCart} label="Nova venda" hint="Registrar uma venda" />}
        {can("products", "write") && <QuickAction to="/produtos?novo=1" icon={Boxes} label="Novo produto" hint="Cadastrar no catálogo" />}
        {can("customers", "write") && <QuickAction to="/clientes?novo=1" icon={UserPlus} label="Novo cliente" hint="Cadastrar cliente" />}
        {canReports && <QuickAction to="/relatorios" icon={BarChart3} label="Relatórios" hint="Vendas, estoque e mapa" />}
      </div>

      {canReports ? <ManagerView /> : can("sales") ? <MyDayView sellerId={me.user.id} /> : null}
      <Alerts />
    </div>
  );
}

function ManagerView() {
  const r = ranges();
  const { data: today } = useAsync(() => reportsApi.sales({ ...r.today, group: "day" }));
  const { data: yesterday } = useAsync(() => reportsApi.sales({ ...r.yesterday, group: "day" }));
  const { data: month } = useAsync(() => reportsApi.sales({ ...r.month, group: "day" }));
  const { data: prevMonth } = useAsync(() => reportsApi.sales({ ...r.prevMonth, group: "day" }));
  const { data: last30 } = useAsync(() => reportsApi.sales(r.last30));
  const { data: prev30 } = useAsync(() => reportsApi.sales(r.prev30));
  const { data: top } = useAsync(() => reportsApi.products({ ...r.month, limit: 5 }));

  const d = (cur?: string | number, before?: string | number, upIsGood = true) =>
    cur != null && before != null ? { pct: pctChange(toNum(cur), toNum(before)), upIsGood, label: "" } : undefined;

  return (
    <>
      <div className="stats">
        <StatTile
          icon={<TrendingUp size={14} aria-hidden />}
          label="Vendido hoje"
          count={toNum(today?.totals.total)}
          format={money}
          delta={today && yesterday ? { ...d(today.totals.total, yesterday.totals.total)!, label: "vs. ontem" } : undefined}
          hint={today ? `${int(today.totals.count)} venda(s)` : undefined}
        />
        <StatTile
          label="No mês"
          count={toNum(month?.totals.total)}
          format={money}
          delta={month && prevMonth ? { ...d(month.totals.total, prevMonth.totals.total)!, label: "vs. mês passado" } : undefined}
          hint={month ? `${int(month.totals.count)} venda(s)` : undefined}
        />
        <StatTile
          label="Ticket médio (mês)"
          count={toNum(month?.totals.avg_ticket)}
          format={money}
          delta={month && prevMonth ? { ...d(month.totals.avg_ticket, prevMonth.totals.avg_ticket)!, label: "vs. mês passado" } : undefined}
        />
        <StatTile
          label="Lucro bruto (mês)"
          count={toNum(month?.totals.profit)}
          format={money}
          delta={month && prevMonth ? { ...d(month.totals.profit, prevMonth.totals.profit)!, label: "vs. mês passado" } : undefined}
          hint={month ? `margem ${toNum(month.totals.margin_pct).toLocaleString("pt-BR")}%` : undefined}
        />
      </div>

      <div className="dash-grid">
        <section className="panel">
          <div className="panel-head">
            <h2>Últimos 30 dias</h2>
            <Link to="/relatorios" className="small">Ver relatório</Link>
          </div>
          {last30 ? (
            <ColumnChart
              ariaLabel="Faturamento dos últimos 30 dias"
              height={210}
              data={last30.series.map((p, i) => ({
                key: p.period,
                label: `${p.period.slice(8, 10)}/${p.period.slice(5, 7)}`,
                value: toNum(p.total),
                previous: prev30 && prev30.series.length === last30.series.length ? toNum(prev30.series[i]?.total) : undefined,
                detail: <span>{int(p.count)} venda(s)</span>,
              }))}
              format={money}
              axisFormat={moneyShort}
            />
          ) : (
            <p className="muted">Carregando…</p>
          )}
        </section>
        <section className="panel">
          <div className="panel-head">
            <h2>Mais vendidos no mês</h2>
          </div>
          {top ? (
            <BarList
              ariaLabel="Cinco produtos mais vendidos no mês"
              data={top.products.map((p) => ({
                key: p.product_id,
                label: p.name,
                value: toNum(p.revenue),
                note: `${qty(p.qty)} ${p.unit}`,
              }))}
              format={money}
              empty="Nenhuma venda neste mês ainda."
            />
          ) : (
            <p className="muted">Carregando…</p>
          )}
        </section>
      </div>
    </>
  );
}

/** Operador: as próprias vendas de hoje. */
function MyDayView({ sellerId }: { sellerId: string }) {
  const r = ranges();
  const { data } = useAsync(() => salesApi.list({ from: r.today.from, to: r.today.to, seller_id: sellerId, status: "concluida" }));
  const total = (data ?? []).reduce((acc, s) => acc + Math.round(toNum(s.total) * 100), 0) / 100;
  return (
    <div className="stats">
      <StatTile icon={<ShoppingCart size={14} aria-hidden />} label="Suas vendas hoje" count={data?.length ?? 0} format={(v) => int(Math.round(v))} />
      <StatTile label="Total vendido por você hoje" count={total} format={money} />
    </div>
  );
}

function Alerts() {
  const { can } = useAuth();
  const navigate = useNavigate();
  const canApprove = can("users", "manage");
  const { data: users } = useAsync(() => (canApprove ? usersApi.list() : Promise.resolve([])), [canApprove]);
  const { data: products } = useAsync(() => (can("products") ? productsApi.list() : Promise.resolve([])));
  const { me } = useAuth();
  const { data: fin } = useAsync(() => (can("finance") ? financeApi.summary(isoDay(new Date())) : Promise.resolve(null)));
  const pending = users?.filter((u) => u.pending).length ?? 0;
  const low = (products ?? []).filter((p) => p.below_min);
  const noLogo = can("settings", "manage") && !me?.company.logo_url;
  const noProducts = can("products", "write") && products != null && products.length === 0;

  const items: { key: string; icon: LucideIcon; tone: "warn" | "info"; text: string; action: string; to: string }[] = [];
  if (pending) items.push({ key: "pending", icon: UserRound, tone: "info", text: `${pending} pedido(s) de acesso aguardando aprovação`, action: "Ver pedidos", to: "/usuarios" });
  if (low.length)
    items.push({
      key: "low",
      icon: AlertTriangle,
      tone: "warn",
      text: `${low.length} produto(s) abaixo do estoque mínimo: ${low.slice(0, 3).map((p) => p.name).join(", ")}${low.length > 3 ? "…" : ""}`,
      action: "Ver produtos",
      to: "/produtos",
    });
  if (fin?.payable_overdue_count)
    items.push({ key: "pay", icon: AlertTriangle, tone: "warn", text: `${fin.payable_overdue_count} conta(s) a pagar vencida(s): ${money(fin.payable_overdue)}`, action: "Ver contas", to: "/financeiro" });
  if (fin?.receivable_overdue_count)
    items.push({ key: "rec", icon: AlertTriangle, tone: "warn", text: `${fin.receivable_overdue_count} conta(s) a receber vencida(s), como fiados: ${money(fin.receivable_overdue)}`, action: "Cobrar", to: "/financeiro" });
  if (noProducts) items.push({ key: "np", icon: Boxes, tone: "info", text: "Cadastre ou importe seus produtos para começar a vender", action: "Ir para produtos", to: "/produtos" });
  if (noLogo) items.push({ key: "logo", icon: CircleCheck, tone: "info", text: "Envie o logo da empresa para aparecer no sistema", action: "Enviar logo", to: "/empresa" });
  if (!items.length) return null;

  return (
    <section className="panel">
      <div className="panel-head">
        <h2>Atenção</h2>
      </div>
      <ul className="alerts">
        {items.map((i) => (
          <li key={i.key} className={`alert alert--${i.tone}`}>
            <i.icon size={18} aria-hidden />
            <span>{i.text}</span>
            <button type="button" className="btn btn--ghost" onClick={() => navigate(i.to)}>
              {i.action}
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
