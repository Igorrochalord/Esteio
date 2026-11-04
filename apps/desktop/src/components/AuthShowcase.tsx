import {
  ClipboardList,
  Crown,
  HardDrive,
  Pause,
  Play,
  QrCode,
  ShoppingCart,
  Wallet,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { authApi } from "../services/endpoints";
import { money, toNum } from "../services/format";
import { PLAN_COPY } from "../services/plans";
import type { PlanItem } from "../services/types";

/** Tempo de cada slide na tela. */
const SLIDE_MS = 6500;

interface Slide {
  key: string;
  icon: LucideIcon;
  kicker: string;
  title: string;
  text: string;
  points: string[];
  price?: string;
}

const ABOUT: Slide[] = [
  {
    key: "vendas",
    icon: ShoppingCart,
    kicker: "Sobre o sistema",
    title: "Vendas e estoque no mesmo lugar",
    text: "Venda com leitor de código de barras, acompanhe o estoque mínimo e veja o lucro de cada venda.",
    points: ["Caixa rápido (F9 finaliza)", "Estoque baixa sozinho", "Relatórios e curva ABC"],
  },
  {
    key: "pix",
    icon: QrCode,
    kicker: "Sobre o sistema",
    title: "PIX com baixa automática",
    text: "O QR Code aparece na tela da venda e o pagamento cai no financeiro sozinho quando o cliente paga.",
    points: ["Mercado Pago, Asaas ou Efí", "Sem digitar nada no banco", "Funciona com a fila off-line"],
  },
  {
    key: "financeiro",
    icon: Wallet,
    kicker: "Sobre o sistema",
    title: "Financeiro sem planilha",
    text: "Contas a pagar e a receber, extrato do banco, fluxo de caixa e DRE, alimentados pelas vendas.",
    points: ["Fiado vira conta a receber", "Importa extrato OFX", "DRE por competência"],
  },
  {
    key: "os",
    icon: ClipboardList,
    kicker: "Sobre o sistema",
    title: "Ordens de serviço no quadro",
    text: "Abra a OS, lance peças e mão de obra, arraste entre as etapas e fature com um clique.",
    points: ["Prazos e técnico responsável", "Imprimir OS", "Faturar gera a venda"],
  },
  {
    key: "local",
    icon: HardDrive,
    kicker: "Sobre o sistema",
    title: "Seus dados ficam com você",
    text: "Tudo fica guardado neste computador, com cópias de segurança automáticas. Sem internet, o sistema continua vendendo.",
    points: ["Cópias automáticas", "Credenciais cifradas", "Internet só para integrações"],
  },
];

function planSlides(plans: PlanItem[]): Slide[] {
  return plans.map((p) => {
    const copy = PLAN_COPY[p.plan];
    const free = toNum(p.price) === 0;
    return {
      key: `plano-${p.plan}`,
      icon: Crown,
      kicker: "Planos",
      title: `Plano ${copy.name}`,
      text: `${copy.tagline}. ${copy.audience}.`,
      points: copy.features.slice(0, 4),
      price: free ? "Grátis" : `${money(p.price)}/mês`,
    };
  });
}

function reducedMotion(): boolean {
  try {
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  } catch {
    return false;
  }
}

/**
 * Vitrine da tela de entrada: alterna entre o que o sistema faz e os planos.
 * Pausa com o mouse em cima, com foco do teclado ou pelo botão; quem prefere menos
 * movimento começa pausado.
 */
export function AuthShowcase() {
  const [plans, setPlans] = useState<PlanItem[]>([]);
  const [index, setIndex] = useState(0);
  const [paused, setPaused] = useState(reducedMotion);
  const [hover, setHover] = useState(false);

  useEffect(() => {
    authApi
      .plans()
      .then(setPlans)
      .catch(() => undefined); // sem planos: mostra só o "sobre"
  }, []);

  // intercala: sobre, plano, sobre, plano…
  const slides = useMemo(() => {
    const ps = planSlides(plans);
    const out: Slide[] = [];
    for (let i = 0; i < Math.max(ABOUT.length, ps.length); i++) {
      if (ABOUT[i]) out.push(ABOUT[i]);
      if (ps[i]) out.push(ps[i]);
    }
    return out;
  }, [plans]);

  const stopped = paused || hover;
  useEffect(() => {
    if (stopped || slides.length < 2) return;
    const t = window.setTimeout(() => setIndex((i) => (i + 1) % slides.length), SLIDE_MS);
    return () => window.clearTimeout(t);
  }, [index, stopped, slides.length]);

  const current = slides[index % slides.length];
  if (!current) return null;
  const Icon = current.icon;

  return (
    <section
      className="showcase"
      aria-roledescription="carrossel"
      aria-label="Conheça o sistema e os planos"
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      onFocus={() => setHover(true)}
      onBlur={() => setHover(false)}
    >
      <article
        key={current.key}
        className={`showcase-slide${current.price ? " showcase-slide--plan" : ""}`}
        aria-roledescription="slide"
        aria-label={`${(index % slides.length) + 1} de ${slides.length}`}
      >
        <span className="showcase-kicker">
          <Icon aria-hidden /> {current.kicker}
        </span>
        <h2 className="showcase-title">{current.title}</h2>
        {current.price && <p className="showcase-price">{current.price}</p>}
        <p className="showcase-text">{current.text}</p>
        <ul className="showcase-points">
          {current.points.map((p) => (
            <li key={p}>{p}</li>
          ))}
        </ul>
      </article>

      <div className="showcase-nav">
        <button
          type="button"
          className="showcase-toggle"
          aria-label={paused ? "Continuar passando" : "Pausar"}
          onClick={() => setPaused((p) => !p)}
        >
          {paused ? <Play aria-hidden /> : <Pause aria-hidden />}
        </button>
        <div className="showcase-dots" role="group" aria-label="Escolher slide">
          {slides.map((s, i) => (
            <button
              key={s.key}
              type="button"
              className={i === index % slides.length ? "is-on" : undefined}
              aria-label={`${s.kicker}: ${s.title}`}
              aria-current={i === index % slides.length}
              onClick={() => setIndex(i)}
            />
          ))}
        </div>
      </div>
      {!stopped && <span key={`bar-${current.key}`} className="showcase-progress" style={{ animationDuration: `${SLIDE_MS}ms` }} aria-hidden />}
    </section>
  );
}
