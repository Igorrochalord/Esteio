import { NavLink, Outlet } from "react-router";

import { BrandMark } from "../components/BrandMark";
import { useAuth } from "../hooks/useAuth";
import { ROLE_LABEL } from "../services/labels";

export function AppLayout() {
  const { me, can, logout } = useAuth();
  if (!me) return null;

  const links = [
    { to: "/", label: "Início", show: true, end: true },
    { to: "/usuarios", label: "Pessoas", show: can("users") },
    { to: "/empresa", label: "Empresa", show: can("settings") },
    { to: "/auditoria", label: "Histórico", show: can("audit") },
  ];

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="sidebar-brand">
          <BrandMark name={me.tenant.name} logoUrl={me.tenant.logo_url} size={44} />
          <span className="sidebar-company">{me.tenant.name}</span>
        </div>
        <nav aria-label="Principal">
          <ul className="sidebar-nav">
            {links
              .filter((l) => l.show)
              .map((l) => (
                <li key={l.to}>
                  <NavLink to={l.to} end={l.end}>
                    {l.label}
                  </NavLink>
                </li>
              ))}
          </ul>
        </nav>
        <div className="sidebar-user">
          <span className="sidebar-user-name">{me.user.name}</span>
          <span className="sidebar-user-role">{ROLE_LABEL[me.user.role]}</span>
          <button type="button" className="sidebar-logout" onClick={() => void logout()}>
            Sair
          </button>
        </div>
      </aside>
      <main className="content">
        <Outlet />
      </main>
    </div>
  );
}
