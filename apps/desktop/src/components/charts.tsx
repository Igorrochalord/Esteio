import { useEffect, useRef, useState, type ReactNode } from "react";

/**
 * Gráficos em SVG, sem biblioteca. Regras (skill dataviz): uma série por gráfico na escala
 * sequencial azul validada; barras ≤ 24px com ponta arredondada de 4px e base reta;
 * grade em linha fina; texto sempre em tinta de texto (nunca na cor da série); tooltip
 * no hover e no foco do teclado; toda informação também está na tabela ao lado.
 */

export const SEQ = {
  100: "#cde2fb",
  150: "#b7d3f6",
  200: "#9ec5f4",
  250: "#86b6ef",
  300: "#6da7ec",
  350: "#5598e7",
  400: "#3987e5",
  450: "#2a78d6",
  500: "#256abf",
  550: "#1c5cab",
  600: "#184f95",
  650: "#104281",
  700: "#0d366b",
} as const;
export const SERIES = SEQ[450];

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
}: {
  data: ColumnDatum[];
  format: (v: number) => string;
  axisFormat?: (v: number) => string;
  height?: number;
  ariaLabel: string;
}) {
  const [ref, width] = useWidth<HTMLDivElement>();
  const [tip, setTip] = useState<Tip | null>(null);
  const [hover, setHover] = useState<number | null>(null);
  const pad = { top: 12, right: 8, bottom: 26, left: 64 };
  const plotW = width - pad.left - pad.right;
  const plotH = height - pad.top - pad.bottom;
  const { top, step } = niceMax(Math.max(0, ...data.map((d) => d.value)));
  const band = data.length ? plotW / data.length : plotW;
  const barW = Math.max(2, Math.min(24, band * 0.62));
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
              <path
                d={columnPath(cx - barW / 2, y(d.value), barW, y(0) - y(d.value), 4)}
                fill={SERIES}
                opacity={hover == null || hover === i ? 1 : 0.55}
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
      {data.map((d) => (
        <li key={d.key} tabIndex={0} title={`${d.label}: ${format(d.value)}${d.note ? ` (${d.note})` : ""}`}>
          <span className="barlist-label">{d.label}</span>
          <span className="barlist-track">
            {/* escala até 62% da trilha: o resto fica para o valor na ponta, mantendo a proporção */}
            <span className="barlist-bar" style={{ width: `${Math.max(0.5, (d.value / max) * 62)}%`, background: SERIES }} />
            <span className="barlist-value">
              {format(d.value)}
              {d.note && <span className="muted"> · {d.note}</span>}
            </span>
          </span>
        </li>
      ))}
    </ol>
  );
}

/** Indicador: rótulo, valor e uma linha de contexto opcional. */
export function StatTile({ label, value, hint, tone }: { label: string; value: string; hint?: ReactNode; tone?: "warn" }) {
  return (
    <div className={tone === "warn" ? "stat stat--warn" : "stat"}>
      <span className="stat-label">{label}</span>
      <span className="stat-value">{value}</span>
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

// rampa ordinal validada (validate_palette --ordinal): começa no tom 250 para não sumir no fundo
const RAMP = [SEQ[250], SEQ[350], SEQ[450], SEQ[550], SEQ[700]];
const NO_DATA = "#f0efec";

/** Cor sequencial pelo valor relativo; texto branco nos tons escuros. */
function rampColor(v: number, max: number) {
  if (v <= 0 || max <= 0) return { fill: NO_DATA, ink: "#52514e" };
  const i = Math.min(RAMP.length - 1, Math.floor((v / max) * RAMP.length - 1e-9));
  return { fill: RAMP[i], ink: i >= 2 ? "#ffffff" : "#0b0b0b" };
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
                  fill={fill}
                  stroke={hover === uf ? "#0b0b0b" : "none"}
                  strokeWidth={hover === uf ? 2 : 0}
                />
                <text x={x + size / 2} y={y + size / 2} dy="0.35em" textAnchor="middle" className="tile-label" fill={ink}>
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
        {RAMP.map((c) => (
          <span key={c} className="scale-step" style={{ background: c }} />
        ))}
        <span className="muted small">mais · máx. {format(max)}</span>
        <span className="scale-step" style={{ background: NO_DATA, marginLeft: "0.75rem" }} />
        <span className="muted small">sem vendas</span>
      </div>
    </div>
  );
}
