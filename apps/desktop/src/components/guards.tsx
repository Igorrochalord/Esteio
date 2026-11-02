import type { ReactNode } from "react";
import { Navigate, useLocation } from "react-router";

import { useAuth } from "../hooks/useAuth";
import type { Level, Module } from "../services/types";

export function RequireAuth({ children }: { children: ReactNode }) {
  const { status } = useAuth();
  const location = useLocation();
  if (status === "loading") return <div className="splash">Carregando…</div>;
  if (status === "anonymous") return <Navigate to="/login" replace state={{ from: location }} />;
  return <>{children}</>;
}

export function RequireAnonymous({ children }: { children: ReactNode }) {
  const { status } = useAuth();
  if (status === "loading") return <div className="splash">Carregando…</div>;
  if (status === "authenticated") return <Navigate to="/" replace />;
  return <>{children}</>;
}

export function RequirePermission({
  module,
  level = "read",
  children,
}: {
  module: Module;
  level?: Level;
  children: ReactNode;
}) {
  const { can } = useAuth();
  if (!can(module, level)) {
    return (
      <div className="page">
        <h1>Sem acesso</h1>
        <p className="muted">Seu papel nesta empresa não permite abrir esta área. Fale com um administrador.</p>
      </div>
    );
  }
  return <>{children}</>;
}
