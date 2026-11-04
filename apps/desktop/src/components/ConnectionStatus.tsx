import { Wifi, WifiOff } from "lucide-react";
import { useEffect, useState } from "react";
import { NavLink } from "react-router";

import { useOnline } from "../hooks/useOnline";
import { integrationsApi } from "../services/endpoints";
import type { QueueSummary } from "../services/types";

export function ConnectionPill() {
  const online = useOnline();
  const Icon = online ? Wifi : WifiOff;
  return (
    <span className={online ? "conn conn--on" : "conn conn--off"} role="status">
      <Icon size={15} aria-hidden />
      {online ? "On-line" : "Sem internet"}
    </span>
  );
}

/** Rodapé do menu: conexão + o que está esperando na fila (só para quem cuida das integrações). */
export function SidebarConnection({ showQueue }: { showQueue: boolean }) {
  const online = useOnline();
  const [summary, setSummary] = useState<QueueSummary | null>(null);

  useEffect(() => {
    if (!showQueue) return;
    let alive = true;
    const load = () =>
      integrationsApi
        .summary()
        .then((s) => alive && setSummary(s))
        .catch(() => {
          /* sem permissão ou sessão encerrada: só não mostra */
        });
    void load();
    const t = window.setInterval(load, 30_000);
    return () => {
      alive = false;
      window.clearInterval(t);
    };
  }, [showQueue]);

  const waiting = summary ? summary.pending + summary.running : 0;
  const failed = summary?.failed ?? 0;
  // sem integrações configuradas e com internet, não há o que avisar
  if (online && !waiting && !failed && !(summary && summary.enabled_integrations > 0)) return null;

  const text = !online
    ? waiting
      ? `Sem internet · ${waiting} na fila`
      : "Sem internet · tudo funciona"
    : failed
      ? `${failed} envio(s) com falha`
      : waiting
        ? `Enviando ${waiting}…`
        : "Integrações em dia";
  const tone = !online || failed ? "conn conn--off" : "conn conn--on";
  const Icon = online ? Wifi : WifiOff;
  const body = (
    <>
      <Icon size={15} aria-hidden /> {text}
    </>
  );
  return showQueue ? (
    <NavLink to="/integracoes" className={`${tone} sidebar-conn`}>
      {body}
    </NavLink>
  ) : (
    <span className={`${tone} sidebar-conn`} role="status">
      {body}
    </span>
  );
}
