import { ArrowDownRight, ArrowUpRight, Minus } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";

/**
 * Gráficos em SVG, sem biblioteca. Regras (skill dataviz): uma série por gráfico na escala
 * sequencial azul validada; barras ≤ 24px com ponta arredondada de 4px e base reta;
 * grade em linha fina; texto sempre em tinta de texto (nunca na cor da série); tooltip
 * no hover e no foco do teclado; toda informação também está na tabela ao lado.
 */

// Cores: variáveis CSS --chart-* e --map-* (styles.css), com variantes clara e escura validadas.

function useWidth<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  const [width, setWidth] = useState(600);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => setWidth(Math.max(240, Math.floor(entry.contentRect.width))));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return [ref, width] as const;
}

/** Passo "redondo" para o eixo: 1, 2 ou 5 × 10^n. */
function niceMax(max: number, ticks = 4): { top: number; step: number } {
  if (max <= 0) return { top: 1, step: 0.25 };
  const raw = max / ticks;
  const pow = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 5, 10].map((m) => m * pow).find((s) => s >= raw) ?? raw;
  return { top: Math.ceil(max / step) * step, step };
}

/** Caminho de barra vertical com ponta arredondada (r) e base reta. */
function columnPath(x: number, y: number, w: number, h: number, r: number) {
  if (h <= 0) return "";
  const rr = Math.min(r, w / 2, h);
  return `M${x},${y + h} V${y + rr} Q${x},${y} ${x + rr},${y} H${x + w - rr} Q${x + w},${y} ${x + w},${y + rr} V${y + h} Z`;
}

interface Tip {
  x: number;
  y: number;
  content: ReactNode;
}

function Tooltip({ tip }: { tip: Tip | null }) {
  if (!tip) return null;
  return (
    <div className="chart-tip" style={{ left: tip.x, top: tip.y }} role="status">
      {tip.content}
    </div>
  );
}

export interface ColumnDatum {
  key: string;
  /** rótulo curto no eixo */
  label: string;
  value: number;
  /** valor do período de comparação na mesma posição (barra cinza ao lado) */
  previous?: number;
  /** linhas extras do tooltip */
  detail?: ReactNode;
}

/** Colunas verticais (série temporal ou categorias ordenadas como horas). */
export function ColumnChart({
  data,
  format,
  axisFormat,
  height = 220,
  ariaLabel,
  legend = ["Período atual", "Período anterior"],
}: {
  data: ColumnDatum[];
  format: (v: number) => string;
  axisFormat?: (v: number) => string;
  height?: number;
  ariaLabel: string;
  /** rótulos da série principal e da cinza (quando há `previous`) */
  legend?: [string, string];
}) {
  const [ref, width] = useWidth<HTMLDivElement>();
  const [tip, setTip] = useState<Tip | null>(null);
  const [hover, setHover] = useState<number | null>(null);
  const pad = { top: 12, right: 8, bottom: 26, left: 64 };
  const plotW = width - pad.left - pad.right;
  const plotH = height - pad.top - pad.bottom;
  const compare = data.some((d) => d.previous != null);
  const { top, step } = niceMax(Math.max(0, ...data.map((d) => Math.max(d.value, d.previous ?? 0))));
  const band = data.length ? plotW / data.length : plotW;
  // com comparação: duas barras finas lado a lado (≤ 12px cada, 2px de respiro)
  const barW = compare ? Math.max(2, Math.min(12, band * 0.32)) : Math.max(2, Math.min(24, band * 0.62));
  const y = (v: number) => pad.top + plotH - (v / top) * plotH;
  const ticks: number[] = [];
  for (let t = 0; t <= top + step / 2; t += step) ticks.push(t);
  // rótulos do eixo X sem encavalar: no máximo um a cada ~56px
  const every = Math.max(1, Math.ceil(56 / band));
  const fmtAxis = axisFormat ?? format;

  const show = (i: number, cx: number) => {
    const d = data[i];
    setHover(i);
    setTip({
      x: Math.min(Math.max(cx, 90), width - 90),
      y: Math.max(y(d.value) - 8, 8),
      content: (
        <>
          <strong>{format(d.value)}</strong>
          <span>{d.label}</span>
          {d.previous != null && <span>{legend[1].toLowerCase()}: {format(d.previous)}</span>}
          {d.detail}
        </>
      ),
    });
  };
  const hide = () => {
    setHover(null);
    setTip(null);
  };

  return (
    <div className="chart" ref={ref}>
      <svg width={width} height={height} role="img" aria-label={ariaLabel} onPointerLeave={hide}>
        {ticks.map((t) => (
          <g key={t}>
            <line className="chart-grid" x1={pad.left} x2={width - pad.right} y1={y(t)} y2={y(t)} />
            <text className="chart-axis" x={pad.left - 8} y={y(t)} dy="0.32em" textAnchor="end">
              {fmtAxis(t)}
            </text>
          </g>
        ))}
        {data.map((d, i) => {
          const cx = pad.left + band * i + band / 2;
          return (
            <g
              key={d.key}
              tabIndex={0}
              role="img"
              aria-label={`${d.label}: ${format(d.value)}`}
              onPointerEnter={() => show(i, cx)}
              onFocus={() => show(i, cx)}
              onBlur={hide}
              className="chart-hit"
            >
              {/* área de toque maior que a barra */}
              <rect x={pad.left + band * i} y={pad.top} width={band} height={plotH} fill="transparent" />
              {compare && (
                <path
                  className="chart-ghost"
                  d={columnPath(cx - barW - 1, y(d.previous ?? 0), barW, y(0) - y(d.previous ?? 0), 3)}
                  style={{ animationDelay: `${Math.min(i * 12, 360)}ms` }}
                />
              )}
              <path
                className="chart-bar"
                d={columnPath(compare ? cx + 1 : cx - barW / 2, y(d.value), barW, y(0) - y(d.value), compare ? 3 : 4)}
                opacity={hover == null || hover === i ? 1 : 0.55}
                style={{ animationDelay: `${Math.min(i * 12, 360)}ms` }}
              />
              {i % every === 0 && (
                <text className="chart-axis" x={cx} y={height - 8} textAnchor="middle">
                  {d.label}
                </text>
              )}
            </g>
          );
        })}
        <line className="chart-base" x1={pad.left} x2={width - pad.right} y1={y(0)} y2={y(0)} />
      </svg>
      {compare && (
        <div className="chart-legend" aria-hidden>
          <span><i className="swatch swatch--series" /> {legend[0]}</span>
          <span><i className="swatch swatch--ghost" /> {legend[1]}</span>
        </div>
      )}
      <Tooltip tip={tip} />
    </div>
  );
}

export interface BarDatum {
  key: string;
  label: string;
  value: number;
  /** texto à direita do valor (ex.: participação) */
  note?: string;
  detail?: ReactNode;
}

/** Barras horizontais ordenadas (ranking), valor na ponta. HTML puro: textos não são cortados. */
export function BarList({
  data,
  format,
  ariaLabel,
  empty = "Sem dados no período.",
}: {
  data: BarDatum[];
  format: (v: number) => string;
  ariaLabel: string;
  empty?: string;
}) {
  const max = Math.max(0, ...data.map((d) => d.value));
  if (!data.length || max <= 0) return <p className="muted">{empty}</p>;
  return (
    <ol className="barlist" aria-label={ariaLabel}>
      {data.map((d, i) => (
        <li key={d.key} tabIndex={0} aria-label={`${d.label}: ${format(d.value)}${d.note ? `, ${d.note}` : ""}`}>
          <span className="barlist-label">
            <span className="barlist-name">{d.label}</span>
            {d.note && <span className="barlist-note">{d.note}</span>}
          </span>
          <span className="barlist-track">
            {/* escala até 70% da trilha: o resto fica para o valor na ponta, mantendo a proporção */}
            <span className="barlist-bar" style={{ width: `${Math.max(0.5, (d.value / max) * 70)}%`, animationDelay: `${Math.min(i * 40, 400)}ms` }} />
            <span className="barlist-value">{format(d.value)}</span>
          </span>
        </li>
      ))}
    </ol>
  );
}

const reducedMotion = () =>
  typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

/** Número que conta de 0 até o valor (700 ms, desacelerando); respeita "reduzir movimento". */
export function CountUp({ value, format }: { value: number; format: (v: number) => string }) {
  const [shown, setShown] = useState(reducedMotion() ? value : 0);
  const from = useRef(0);
  useEffect(() => {
    if (reducedMotion()) {
      setShown(value);
      return;
    }
    const start = performance.now();
    const origin = from.current;
    let frame = 0;
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / 700);
      const eased = 1 - (1 - t) ** 3;
      setShown(origin + (value - origin) * eased);
      if (t < 1) frame = requestAnimationFrame(tick);
      else from.current = value;
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [value]);
  return <>{format(shown)}</>;
}

export interface Delta {
  /** variação em % (null = sem base de comparação) */
  pct: number | null;
  /** subir é bom? (faturamento sim; cancelamentos não) */
  upIsGood?: boolean;
  label?: string;
}

/** Variação com seta + texto (nunca só cor) e cor de status conforme subir ser bom ou ruim. */
export function DeltaBadge({ delta }: { delta: Delta }) {
  const { pct, upIsGood = true, label = "vs. anterior" } = delta;
  if (pct == null || !Number.isFinite(pct)) return <span className="delta delta--flat">sem comparação</span>;
  const flat = Math.abs(pct) < 0.05;
  const up = pct > 0;
  const tone = flat ? "flat" : up === upIsGood ? "good" : "bad";
  const Icon = flat ? Minus : up ? ArrowUpRight : ArrowDownRight;
  const text = `${up ? "+" : ""}${pct.toLocaleString("pt-BR", { maximumFractionDigits: 1, minimumFractionDigits: 1 })}%`;
  return (
    <span className={`delta delta--${tone}`}>
      <Icon size={14} aria-hidden />
      {text} <span className="delta-label">{label}</span>
    </span>
  );
}

/** Indicador: rótulo, valor (contando, se numérico), contexto e variação opcionais. */
export function StatTile({
  label,
  value,
  count,
  format,
  hint,
  tone,
  delta,
  icon,
}: {
  label: string;
  value?: string;
  /** número animado (usa `format`) */
  count?: number;
  format?: (v: number) => string;
  hint?: ReactNode;
  tone?: "warn";
  delta?: Delta;
  icon?: ReactNode;
}) {
  return (
    <div className={tone === "warn" ? "stat stat--warn" : "stat"}>
      <span className="stat-label">
        {icon}
        {label}
      </span>
      <span className="stat-value">{count != null && format ? <CountUp value={count} format={format} /> : value}</span>
      {delta && <DeltaBadge delta={delta} />}
      {hint && <span className="stat-hint">{hint}</span>}
    </div>
  );
}

// ------------------------------------------------------------------ mapa em blocos

/** Posição (coluna, linha) de cada UF num mapa em blocos do Brasil. */
const TILES: Record<string, [number, number]> = {
  RR: [1, 0], AP: [3, 0],
  AM: [1, 1], PA: [2, 1], MA: [3, 1], CE: [4, 1], RN: [5, 1],
  AC: [0, 2], RO: [1, 2], MT: [2, 2], TO: [3, 2], PI: [4, 2], PB: [5, 2],
  MS: [2, 3], GO: [3, 3], BA: [4, 3], PE: [5, 3],
  SP: [2, 4], DF: [3, 4], MG: [4, 4], AL: [5, 4],
  PR: [2, 5], RJ: [3, 5], ES: [4, 5], SE: [5, 5],
  SC: [2, 6],
  RS: [2, 7],
};

// rampa ordinal de 5 faixas (validate_palette --ordinal, claro e escuro): --map-1 … --map-5
const STEPS = 5;

/** Faixa pelo valor relativo; texto da faixa vem de --map-ink-N (contraste garantido). */
function rampColor(v: number, max: number) {
  if (v <= 0 || max <= 0) return { fill: "var(--chart-nodata)", ink: "var(--muted)" };
  const i = Math.min(STEPS - 1, Math.floor((v / max) * STEPS - 1e-9)) + 1;
  return { fill: `var(--map-${i})`, ink: `var(--map-ink-${i})` };
}

export interface StateDatum {
  state: string;
  value: number;
  detail?: ReactNode;
}

export function BrazilTileMap({
  data,
  format,
  ariaLabel,
}: {
  data: StateDatum[];
  format: (v: number) => string;
  ariaLabel: string;
}) {
  const [tip, setTip] = useState<Tip | null>(null);
  const [hover, setHover] = useState<string | null>(null);
  const byState = new Map(data.map((d) => [d.state, d]));
  const max = Math.max(0, ...data.map((d) => d.value));
  const size = 46;
  const gap = 4;
  const w = 6 * (size + gap);
  const h = 8 * (size + gap);

  const show = (uf: string, x: number, y: number) => {
    const d = byState.get(uf);
    setHover(uf);
    setTip({
      x: x + size / 2,
      y,
      content: d ? (
        <>
          <strong>{format(d.value)}</strong>
          <span>{uf}</span>
          {d.detail}
        </>
      ) : (
        <>
          <strong>Sem vendas</strong>
          <span>{uf}</span>
        </>
      ),
    });
  };

  return (
    <div className="tilemap">
      <div className="chart" style={{ width: w }}>
        <svg width={w} height={h} role="img" aria-label={ariaLabel} onPointerLeave={() => (setTip(null), setHover(null))}>
          {Object.entries(TILES).map(([uf, [c, r]]) => {
            const d = byState.get(uf);
            const { fill, ink } = rampColor(d?.value ?? 0, max);
            const x = c * (size + gap);
            const y = r * (size + gap);
            return (
              <g
                key={uf}
                tabIndex={0}
                role="img"
                aria-label={`${uf}: ${d ? format(d.value) : "sem vendas"}`}
                onPointerEnter={() => show(uf, x, y)}
                onFocus={() => show(uf, x, y)}
                onBlur={() => (setTip(null), setHover(null))}
                className="chart-hit"
              >
                <rect
                  x={x}
                  y={y}
                  width={size}
                  height={size}
                  rx={4}
                  style={{ fill, stroke: hover === uf ? "var(--ink-strong)" : "none", strokeWidth: hover === uf ? 2 : 0 }}
                />
                <text x={x + size / 2} y={y + size / 2} dy="0.35em" textAnchor="middle" className="tile-label" style={{ fill: ink }}>
                  {uf}
                </text>
              </g>
            );
          })}
        </svg>
        <Tooltip tip={tip} />
      </div>
      <div className="scale-legend" aria-hidden>
        <span className="muted small">menos</span>
        {Array.from({ length: STEPS }, (_, k) => (
          <span key={k} className="scale-step" style={{ background: `var(--map-${k + 1})` }} />
        ))}
        <span className="muted small">mais · máx. {format(max)}</span>
        <span className="scale-step" style={{ background: "var(--chart-nodata)", marginLeft: "0.75rem" }} />
        <span className="muted small">sem vendas</span>
      </div>
    </div>
  );
}
