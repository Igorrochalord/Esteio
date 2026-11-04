import { Sparkles, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router";

/** Intervalo do lembrete de planos (pedido do produto: a cada 5 minutos com sessão aberta). */
export const PLAN_NUDGE_INTERVAL_MS = 5 * 60 * 1000;

/**
 * Lembrete discreto no canto da tela. Regras para não atrapalhar o trabalho:
 * só para quem pode assinar (dono/admin), nunca por cima de um diálogo aberto (ex.: uma venda),
 * nem na própria página de planos.
 */
export function PlansNudge({ enabled, paused }: { enabled: boolean; paused: boolean }) {
  const [visible, setVisible] = useState(false);
  const navigate = useNavigate();
  const { pathname } = useLocation();

  useEffect(() => {
    if (!enabled) return;
    const timer = window.setInterval(() => {
      const busy = paused || !!document.querySelector("dialog[open]") || window.location.hash.startsWith("#/planos");
      if (!busy) setVisible(true);
    }, PLAN_NUDGE_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [enabled, paused]);

  useEffect(() => {
    if (pathname === "/planos") setVisible(false);
  }, [pathname]);

  if (!visible) return null;
  return (
    <aside className="nudge" role="status" aria-live="polite">
      <span className="nudge-icon">
        <Sparkles size={20} aria-hidden />
      </span>
      <div>
        <strong>Conheça os planos do Núcleo ERP</strong>
        <p className="muted small">Hoje tudo está liberado. Veja o que cada plano vai oferecer: NFS-e, ordens de serviço, DRE e mais.</p>
        <div className="row">
          <button
            type="button"
            className="btn btn--primary"
            onClick={() => {
              setVisible(false);
              navigate("/planos");
            }}
          >
            Ver planos
          </button>
          <button type="button" className="btn btn--ghost" onClick={() => setVisible(false)}>
            Agora não
          </button>
        </div>
      </div>
      <button type="button" className="icon-btn nudge-close" aria-label="Fechar" onClick={() => setVisible(false)}>
        <X size={16} aria-hidden />
      </button>
    </aside>
  );
}
