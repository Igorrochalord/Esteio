import { ArrowDownCircle, ArrowUpCircle, Banknote, Building, Landmark, Wallet, type LucideIcon } from "lucide-react";
import { useState } from "react";

import { ColumnChart, StatTile } from "../../components/charts";
import { useAsync } from "../../hooks/useAsync";
import { financeApi } from "../../services/endpoints";
import { date, isoDay, money, moneyShort, toNum } from "../../services/format";
import { ACCOUNT_KIND_LABEL } from "../../services/labels";
import type { BankAccount, FinEntry } from "../../services/types";
import { EntriesTab } from "./Entries";
import { CashFlowReport, DreReport } from "./FinReports";
import { AccountsTab, CategoriesTab } from "./Setup";
import { StatementTab } from "./Statement";

type Tab = "overview" | "receber" | "pagar" | "extrato" | "fluxo" | "dre" | "contas" | "categorias";
const TABS: { key: Tab; label: string }[] = [
  { key: "overview", label: "Visão geral" },
  { key: "receber", label: "A receber" },
  { key: "pagar", label: "A pagar" },
  { key: "extrato", label: "Extrato" },
  { key: "fluxo", label: "Fluxo de caixa" },
  { key: "dre", label: "DRE" },
  { key: "contas", label: "Contas" },
  { key: "categorias", label: "Categorias" },
];
const KIND_ICON: Record<string, LucideIcon> = { caixa: Banknote, corrente: Landmark, poupanca: Building, carteira: Wallet };

export function FinancePage() {
  const [tab, setTab] = useState<Tab>("overview");
  const { data: accounts, reload: reloadAccounts } = useAsync(financeApi.accounts);
  const { data: categories, reload: reloadCategories } = useAsync(financeApi.categories);
  const [version, setVersion] = useState(0);
  const changed = async () => {
    await reloadAccounts();
    setVersion((v) => v + 1);
  };

  return (
    <div className="page page--wide">
      <header className="page-header">
        <div>
          <h1>Financeiro</h1>
          <p className="muted">Contas, a pagar e a receber, extrato, fluxo de caixa e DRE. Vendas à vista e fiado entram aqui sozinhas.</p>
        </div>
      </header>
      <nav className="tabs no-print" role="tablist" aria-label="Financeiro">
        {TABS.map((t) => (
          <button key={t.key} type="button" role="tab" aria-selected={tab === t.key} className={tab === t.key ? "tab is-on" : "tab"} onClick={() => setTab(t.key)}>
            {t.label}
          </button>
        ))}
      </nav>
      <div role="tabpanel" key={`${tab}-${tab === "overview" ? version : 0}`}>
        {tab === "overview" && <Overview accounts={accounts ?? []} onOpen={setTab} />}
        {(tab === "receber" || tab === "pagar") && (
          <EntriesTab kind={tab} accounts={accounts ?? []} categories={categories ?? []} onChanged={() => void changed()} />
        )}
        {tab === "extrato" && <StatementTab accounts={accounts ?? []} categories={categories ?? []} onChanged={() => void changed()} />}
        {tab === "fluxo" && <CashFlowReport />}
        {tab === "dre" && <DreReport />}
        {tab === "contas" && <AccountsTab accounts={accounts ?? []} onChanged={changed} />}
        {tab === "categorias" && <CategoriesTab categories={categories ?? []} onChanged={async () => { await reloadCategories(); }} />}
      </div>
    </div>
  );
}

function Overview({ accounts, onOpen }: { accounts: BankAccount[]; onOpen: (t: Tab) => void }) {
  const today = isoDay(new Date());
  const in7 = isoDay(new Date(Date.now() + 7 * 86_400_000));
  const start = isoDay(new Date(Date.now() - 29 * 86_400_000));
  const { data: s } = useAsync(() => financeApi.summary(today));
  const { data: upcoming } = useAsync(() => financeApi.entries({ status: "aberto", due_to: in7 }));
  const { data: flow } = useAsync(() => financeApi.cashFlow({ from: start, to: today, group: "day" }));
  const active = accounts.filter((a) => a.active);

  return (
    <div className="report">
      <div className="stats">
        <StatTile icon={<Wallet size={14} aria-hidden />} label="Saldo em contas" count={toNum(s?.total_balance)} format={money} />
        <StatTile icon={<ArrowDownCircle size={14} aria-hidden />} label="A receber vencido" count={toNum(s?.receivable_overdue)} format={money}
          tone={s?.receivable_overdue_count ? "warn" : undefined} hint={s ? `${s.receivable_overdue_count} título(s) · próximos 7 dias: ${money(s.receivable_next7)}` : undefined} />
        <StatTile icon={<ArrowUpCircle size={14} aria-hidden />} label="A pagar vencido" count={toNum(s?.payable_overdue)} format={money}
          tone={s?.payable_overdue_count ? "warn" : undefined} hint={s ? `${s.payable_overdue_count} título(s) · hoje: ${money(s.payable_today)}` : undefined} />
        <StatTile label="A pagar nos próximos 7 dias" count={toNum(s?.payable_next7)} format={money} />
      </div>

      <div className="account-cards">
        {active.map((a, i) => {
          const Icon = KIND_ICON[a.kind] ?? Landmark;
          return (
            <button key={a.id} type="button" className="account-card" style={{ animationDelay: `${i * 50}ms` }} onClick={() => onOpen("extrato")}>
              <span className="quick-icon"><Icon size={20} aria-hidden /></span>
              <span>
                <span className="muted small">{ACCOUNT_KIND_LABEL[a.kind]}{a.is_default ? " · recebe vendas" : ""}</span>
                <strong>{a.name}</strong>
                <span className={toNum(a.balance) < 0 ? "account-balance neg" : "account-balance"}>{money(a.balance)}</span>
              </span>
            </button>
          );
        })}
        {!active.length && <p className="muted">Nenhuma conta ainda. Crie em Contas, ou faça uma venda à vista: a conta Caixa é criada sozinha.</p>}
      </div>

      <div className="dash-grid">
        <section className="panel">
          <div className="panel-head"><h2>Entradas × saídas (30 dias)</h2></div>
          {flow ? (
            <ColumnChart
              ariaLabel="Entradas e saídas dos últimos 30 dias"
              height={200}
              legend={["Entradas", "Saídas"]}
              data={flow.rows.map((r) => ({ key: r.period, label: `${r.period.slice(8, 10)}/${r.period.slice(5, 7)}`, value: toNum(r.realized_in), previous: toNum(r.realized_out) }))}
              format={money}
              axisFormat={moneyShort}
            />
          ) : (
            <p className="muted">Carregando…</p>
          )}
        </section>
        <section className="panel">
          <div className="panel-head">
            <h2>Vencidos e próximos 7 dias</h2>
          </div>
          <UpcomingList entries={upcoming ?? []} today={today} onOpen={onOpen} />
        </section>
      </div>
    </div>
  );
}

function UpcomingList({ entries, today, onOpen }: { entries: FinEntry[]; today: string; onOpen: (t: Tab) => void }) {
  if (!entries.length) return <p className="muted">Nada vencido nem vencendo nos próximos dias. 🎉</p>;
  return (
    <ul className="upcoming">
      {entries.slice(0, 12).map((e) => {
        const late = e.due_date < today;
        return (
          <li key={e.id}>
            <button type="button" onClick={() => onOpen(e.kind)}>
              <span className={e.kind === "receber" ? "dot dot--in" : "dot dot--out"} aria-hidden />
              <span className="upcoming-text">
                <strong>{e.description}</strong>
                <span className="muted small">
                  {e.kind === "receber" ? "A receber" : "A pagar"} · {e.customer_name ?? e.counterparty ?? e.category_name}
                </span>
              </span>
              <span className="upcoming-right">
                <strong className={e.kind === "pagar" ? "neg" : "pos"}>{money(e.amount)}</strong>
                <span className={late ? "tag tag--danger" : e.due_date === today ? "tag tag--warn" : "tag"}>
                  {late ? "venceu " : e.due_date === today ? "" : "vence "}
                  {e.due_date === today ? "hoje" : date(`${e.due_date}T12:00:00`)}
                </span>
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
