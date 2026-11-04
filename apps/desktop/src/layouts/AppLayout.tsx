import {
  BarChart3,
  Boxes,
  Building2,
  CircleHelp,
  ClipboardList,
  DatabaseBackup,
  Gem,
  History,
  House,
  Landmark,
  LogOut,
  Monitor,
  Moon,
  Plug,
  ShoppingCart,
  Sparkles,
  Sun,
  UserRound,
  Users,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState } from "react";
import { NavLink, Outlet, useLocation } from "react-router";

import { BrandMark } from "../components/BrandMark";
import { SidebarConnection } from "../components/ConnectionStatus";
import { PlansNudge } from "../components/PlansNudge";
import { Tour } from "../components/Tour";
import { useAuth } from "../hooks/useAuth";
import { ROLE_LABEL } from "../services/labels";
import { initials } from "../services/theme";
import { applyThemeMode, getThemeMode, nextThemeMode, type ThemeMode } from "../services/themeMode";

interface Link {
  to: string;
  label: string;
  icon: LucideIcon;
  show: boolean;
  end?: boolean;
}

const THEME_LABEL: Record<ThemeMode, string> = { system: "Tema automático", light: "Tema claro", dark: "Tema escuro" };
const THEME_ICON: Record<ThemeMode, LucideIcon> = { system: Monitor, light: Sun, dark: Moon };

export function AppLayout() {
  const { me, can, logout } = useAuth();
  const location = useLocation();
  const [theme, setTheme] = useState<ThemeMode>(getThemeMode);
  const [tourOpen, setTourOpen] = useState(false);
  const tourKey = me ? `nucleo.tour.v1.${me.user.id}` : "";

  // tutorial abre sozinho no primeiro acesso de cada pessoa neste computador
  useEffect(() => {
    if (!tourKey) return;
    let seen = true;
    try {
      seen = localStorage.getItem(tourKey) === "1";
    } catch {
      /* sem armazenamento: não insiste */
    }
    if (!seen) {
      const t = window.setTimeout(() => setTourOpen(true), 700);
      return () => window.clearTimeout(t);
    }
  }, [tourKey]);

  const closeTour = () => {
    setTourOpen(false);
    try {
      localStorage.setItem(tourKey, "1");
    } catch {
      /* ignora */
    }
  };

  if (!me) return null;
  const canBuy = me.user.role === "owner" || me.user.role === "admin";

  const sections: { title?: string; links: Link[] }[] = [
    {
      links: [
        { to: "/", label: "Início", icon: House, show: true, end: true },
        { to: "/vendas", label: "Vendas", icon: ShoppingCart, show: can("sales") },
        { to: "/produtos", label: "Produtos", icon: Boxes, show: can("products") },
        { to: "/clientes", label: "Clientes", icon: UserRound, show: can("customers") },
        { to: "/os", label: "Ordens de serviço", icon: ClipboardList, show: can("service_orders") },
        { to: "/financeiro", label: "Financeiro", icon: Landmark, show: can("finance") },
        { to: "/relatorios", label: "Relatórios", icon: BarChart3, show: can("reports") },
      ],
    },
    {
      title: "Administração",
      links: [
        { to: "/usuarios", label: "Pessoas", icon: Users, show: can("users") },
        { to: "/empresa", label: "Empresa", icon: Building2, show: can("settings") },
        { to: "/auditoria", label: "Histórico", icon: History, show: can("audit") },
        { to: "/integracoes", label: "Integrações", icon: Plug, show: can("integrations") },
        { to: "/backup", label: "Cópias de segurança", icon: DatabaseBackup, show: can("backup", "write") },
        { to: "/planos", label: "Planos", icon: Gem, show: canBuy },
      ],
    },
  ];
  const ThemeIcon = THEME_ICON[theme];

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="sidebar-brand">
          <BrandMark name={me.company.name} logoUrl={me.company.logo_url} size={36} />
          <span className="sidebar-company">{me.company.name}</span>
        </div>
        <nav aria-label="Principal">
          {sections.map((section, i) => {
            const links = section.links.filter((l) => l.show);
            if (!links.length) return null;
            return (
              <div key={i}>
                {section.title && <div className="sidebar-section">{section.title}</div>}
                <ul className="sidebar-nav">
                  {links.map((l) => (
                    <li key={l.to}>
                      <NavLink to={l.to} end={l.end}>
                        <l.icon size={18} strokeWidth={1.9} aria-hidden />
                        {l.label}
                      </NavLink>
                    </li>
                  ))}
                </ul>
              </div>
            );
          })}
        </nav>
        <SidebarConnection showQueue={can("integrations")} />
        <button type="button" className="sidebar-help" onClick={() => setTourOpen(true)}>
          <CircleHelp size={18} strokeWidth={1.9} aria-hidden /> Como usar
        </button>
        {me.plan && (
          <NavLink to={canBuy ? "/planos" : "/"} className="plan-badge" title={me.plan.note}>
            <Sparkles size={16} aria-hidden />
            <span>
              <strong>{me.plan.label}</strong>
              {me.plan.unlimited ? "Tudo liberado no lançamento" : me.plan.note}
            </span>
          </NavLink>
        )}
        <div className="sidebar-user">
          <span className="avatar" aria-hidden>{initials(me.user.name) || "?"}</span>
          <span>
            <span className="sidebar-user-name">{me.user.name}</span>
            <br />
            <span className="sidebar-user-role">{ROLE_LABEL[me.user.role]}</span>
          </span>
          <span className="row" style={{ gap: 2 }}>
            <button
              type="button"
              className="icon-btn"
              title={`${THEME_LABEL[theme]} (clique para trocar)`}
              aria-label={`${THEME_LABEL[theme]}. Trocar tema`}
              onClick={() => {
                const next = nextThemeMode(theme);
                applyThemeMode(next);
                setTheme(next);
              }}
            >
              <ThemeIcon size={17} aria-hidden />
            </button>
            <button type="button" className="icon-btn" title="Sair" aria-label="Sair" onClick={() => void logout()}>
              <LogOut size={17} aria-hidden />
            </button>
          </span>
        </div>
      </aside>
      <Tour open={tourOpen} role={me.user.role} firstName={me.user.name.split(" ")[0]} company={me.company.name} onClose={closeTour} />
      <PlansNudge enabled={canBuy} paused={tourOpen} />
      <main className="content">
        {/* a key reinicia a animação de entrada a cada troca de tela */}
        <div key={location.pathname}>
          <Outlet />
        </div>
      </main>
    </div>
  );
}
