import { Check, CreditCard, Gift, RefreshCw, ShieldCheck, Sparkles, X, XCircle, type LucideIcon } from "lucide-react";
import { useMemo, useState } from "react";

import { CountUp } from "../components/charts";
import { useAsync } from "../hooks/useAsync";
import { useAuth } from "../hooks/useAuth";
import { authApi } from "../services/endpoints";
import { money, toNum } from "../services/format";
import type { PlanCode, PlanItem } from "../services/types";

/** Textos dos planos (preços e limites vêm do núcleo: plans.rs). */
const COPY: Record<PlanCode, { name: string; tagline: string; audience: string; features: string[]; missing?: string[] }> = {
  gratuito: {
    name: "Gratuito",
    tagline: "Para organizar a gestão",
    audience: "Pessoa física ou jurídica",
    features: ["Gestão financeira", "Cadastros, vendas e estoque", "Até 3 contas bancárias", "Usuários ilimitados", "Suporte por e-mail"],
    missing: ["Emissão de NFS-e", "Ordens de serviço"],
  },
  starter: {
    name: "Starter",
    tagline: "Para quem está começando",
    audience: "Autônomos e pequenas empresas",
    features: [
      "Tudo do Gratuito",
      "Emissão de NFS-e (25 por mês)",
      "Ordens de serviço ilimitadas",
      "Contas bancárias ilimitadas",
      "Conciliação bancária",
      "Até 5 CNPJs adicionais",
      "Suporte por e-mail, WhatsApp e telefone",
    ],
  },
  platinum: {
    name: "Platinum",
    tagline: "Para crescer com volume",
    audience: "Empresas em expansão",
    features: ["Tudo do Starter", "100 NFS-e por mês", "Relatórios avançados com DRE", "Até 10 CNPJs adicionais", "Suporte prioritário"],
  },
  infinite: {
    name: "Infinite",
    tagline: "Para quem não pode parar",
    audience: "Alto volume e atendimento exclusivo",
    features: ["Tudo do Platinum", "1.000 NFS-e por mês", "CNPJs adicionais ilimitados", "Suporte prioritário dedicado"],
  },
};

const EXTRA_CNPJ = 19.9;

const STEPS: { icon: LucideIcon; title: string; text: string }[] = [
  { icon: Gift, title: "Comece grátis", text: "Sem cartão de crédito. Quando quiser, assine um plano pago dentro do próprio sistema." },
  { icon: CreditCard, title: "Pague como preferir", text: "Pix, boleto ou cartão. O plano é liberado assim que o pagamento é confirmado." },
  { icon: RefreshCw, title: "Renovação mensal", text: "Mensalidade fixa, renovada automaticamente. Troque de plano quando precisar." },
  { icon: ShieldCheck, title: "Garantia de 7 dias", text: "Não se adaptou? Devolvemos 100% do valor pago nos primeiros 7 dias." },
  { icon: XCircle, title: "Cancele quando quiser", text: "O acesso segue até o fim do período pago e depois a conta volta ao Gratuito, sem perder dados." },
];

/** Custo mensal do plano para o cenário, ou null se o plano não atende. */
function costFor(p: PlanItem, cnpjs: number, nfse: number): number | null {
  const extra = cnpjs - 1;
  const l = p.limits;
  if (nfse > 0 && !l.nfse) return null;
  if (l.nfse_per_month != null && nfse > l.nfse_per_month) return null;
  if (l.extra_cnpjs != null && extra > l.extra_cnpjs) return null;
  return toNum(p.price) + extra * EXTRA_CNPJ;
}

export function PlansPage() {
  const { me } = useAuth();
  const { data: plans, error } = useAsync(authApi.plans);
  const [cnpjs, setCnpjs] = useState(1);
  const [nfse, setNfse] = useState(25);

  const sim = useMemo(() => {
    const rows = (plans ?? []).map((p) => ({ p, cost: costFor(p, cnpjs, nfse) }));
    const best = rows.filter((r) => r.cost != null).sort((a, b) => (a.cost ?? 0) - (b.cost ?? 0))[0];
    return { rows, best: best?.p.plan };
  }, [plans, cnpjs, nfse]);

  return (
    <div className="page page--wide plans-page">
      <header className="plans-hero">
        <span className="hero-badge">
          <Sparkles size={15} aria-hidden /> {me?.plan.label ?? "Acesso completo"}
        </span>
        <h1>Planos do Núcleo ERP</h1>
        <p className="muted">
          Hoje <strong>todos os recursos estão liberados</strong> no lançamento. Quando os planos começarem a valer,
          você será avisado com antecedência e seus dados continuam sempre seus.
        </p>
      </header>

      {error && <p className="field-error">{error}</p>}

      <div className="plan-cards">
        {(plans ?? []).map((p, i) => {
          const c = COPY[p.plan];
          const popular = p.plan === "platinum";
          return (
            <article key={p.plan} className={popular ? "plan-card is-popular" : "plan-card"} style={{ animationDelay: `${i * 70}ms` }}>
              {popular && <span className="plan-ribbon">Mais popular</span>}
              <h2>{c.name}</h2>
              <p className="muted small">{c.tagline}</p>
              <p className="plan-price">
                {toNum(p.price) === 0 ? (
                  "Grátis"
                ) : (
                  <>
                    <CountUp value={toNum(p.price)} format={(v) => money(v).replace(",00", "")} />
                    <span className="muted small">/mês</span>
                  </>
                )}
              </p>
              <p className="muted small">{c.audience}</p>
              <ul className="plan-features">
                {c.features.map((f) => (
                  <li key={f}>
                    <Check size={16} aria-hidden /> {f}
                  </li>
                ))}
                {c.missing?.map((f) => (
                  <li key={f} className="is-missing">
                    <X size={16} aria-hidden /> {f}
                  </li>
                ))}
              </ul>
              <button className={popular ? "btn btn--primary btn--block" : "btn btn--ghost btn--block"} disabled title="Disponível quando a cobrança começar">
                Disponível em breve
              </button>
            </article>
          );
        })}
      </div>

      <section className="panel">
        <h2>Simule o seu plano</h2>
        <p className="muted">Informe quantos CNPJs você gerencia e quantas NFS-e emite por mês. Mostramos o plano mais barato que atende.</p>
        <div className="sim-grid">
          <label className="sim-field">
            <span>
              CNPJs gerenciados: <strong>{cnpjs}</strong>
            </span>
            <input type="range" min={1} max={20} value={cnpjs} onChange={(e) => setCnpjs(Number(e.target.value))} />
            <span className="muted small">Cada CNPJ além do principal: {money(EXTRA_CNPJ)}/mês</span>
          </label>
          <label className="sim-field">
            <span>
              NFS-e por mês: <strong>{nfse >= 1000 ? "1.000" : nfse}</strong>
            </span>
            <input type="range" min={0} max={1000} step={5} value={nfse} onChange={(e) => setNfse(Number(e.target.value))} />
            <span className="muted small">Nota fiscal de serviço eletrônica</span>
          </label>
        </div>
        <div className="sim-results">
          {sim.rows.map(({ p, cost }) => (
            <div key={p.plan} className={sim.best === p.plan ? "sim-card is-best" : cost == null ? "sim-card is-out" : "sim-card"}>
              <span className="sim-name">
                {COPY[p.plan].name} {sim.best === p.plan && <span className="tag tag--ok">Melhor opção</span>}
              </span>
              <span className="sim-cost">{cost == null ? "Não atende" : `${money(cost)}/mês`}</span>
            </div>
          ))}
        </div>
        <p className="muted small">Simulação estimada. Valores de referência, sujeitos às condições comerciais no lançamento dos planos.</p>
      </section>

      <section className="panel">
        <h2>Compare os planos</h2>
        <div className="table-wrap">
          <table className="table compare-table">
            <thead>
              <tr>
                <th>Recurso</th>
                {(plans ?? []).map((p) => (
                  <th key={p.plan}>
                    {COPY[p.plan].name}
                    <div className="muted small">{toNum(p.price) ? `${money(p.price)}/mês` : "R$ 0"}</div>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {[
                ["Gestão financeira, cadastros, vendas e estoque", () => true],
                ["Usuários", () => "Ilimitados"],
                ["Contas bancárias", (p: PlanItem) => (p.limits.bank_accounts == null ? "Ilimitadas" : `Até ${p.limits.bank_accounts}`)],
                ["Ordens de serviço", (p: PlanItem) => p.limits.service_orders && "Ilimitadas"],
                ["NFS-e por mês", (p: PlanItem) => (p.limits.nfse ? `${p.limits.nfse_per_month?.toLocaleString("pt-BR")}` : false)],
                ["Conciliação bancária", (p: PlanItem) => p.limits.bank_reconciliation],
                ["Relatórios avançados (DRE)", (p: PlanItem) => p.limits.advanced_reports],
                ["CNPJs adicionais", (p: PlanItem) => (p.limits.extra_cnpjs == null ? "Ilimitados" : p.limits.extra_cnpjs === 0 ? false : `Até ${p.limits.extra_cnpjs}`)],
              ].map(([label, fn]) => (
                <tr key={label as string}>
                  <td>{label as string}</td>
                  {(plans ?? []).map((p) => {
                    const v = (fn as (p: PlanItem) => string | boolean)(p);
                    return (
                      <td key={p.plan} className="compare-cell">
                        {v === true ? <Check size={18} className="yes" aria-label="Sim" /> : v === false ? <span className="muted" aria-label="Não">—</span> : v}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <section className="panel">
        <h2>Como vai funcionar</h2>
        <ol className="how-steps">
          {STEPS.map((s, i) => (
            <li key={s.title} style={{ animationDelay: `${i * 60}ms` }}>
              <span className="how-icon">
                <s.icon size={20} aria-hidden />
              </span>
              <strong>{s.title}</strong>
              <span className="muted small">{s.text}</span>
            </li>
          ))}
        </ol>
      </section>
    </div>
  );
}
