import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";

import { AuthContext, type AuthState, type AuthStatus } from "../hooks/authContext";
import { onSessionExpired } from "../services/api";
import { authApi } from "../services/endpoints";
import { applyBranding } from "../services/theme";
import type { Level, Me, Module } from "../services/types";

const RANK: Record<Level, number> = { read: 1, write: 2, manage: 3 };

export function AuthProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<AuthStatus>("loading");
  const [me, setMe] = useState<Me | null>(null);

  const startSession = useCallback((data: Me) => {
    setMe(data);
    setStatus("authenticated");
    applyBranding(data.company);
  }, []);

  /** Sem sessão: vai para o login, ou para o onboarding se ainda não há empresa. */
  const endSession = useCallback(() => {
    setMe(null);
    applyBranding(null);
    setStatus("loading");
    authApi
      .setupStatus()
      .then((s) => setStatus(s.configured ? "anonymous" : "setup"))
      .catch(() => setStatus("anonymous"));
  }, []);

  const loadMe = useCallback(async () => startSession(await authApi.me()), [startSession]);

  // ao abrir: a sessão vive no processo do app, então sobrevive a recarregar a tela
  useEffect(() => {
    authApi.me().then(startSession, endSession);
  }, [startSession, endSession]);

  useEffect(() => onSessionExpired(endSession), [endSession]);

  const login = useCallback(
    async (email: string, password: string) => startSession(await authApi.login(email, password)),
    [startSession],
  );

  const logout = useCallback(async () => {
    await authApi.logout().catch(() => undefined);
    endSession();
  }, [endSession]);

  const can = useCallback(
    (module: Module, level: Level = "read") => {
      const granted = me?.permissions[module];
      return !!granted && RANK[granted] >= RANK[level];
    },
    [me],
  );

  const value = useMemo<AuthState>(
    () => ({ status, me, login, startSession, logout, reloadMe: loadMe, endSession, can }),
    [status, me, login, startSession, logout, loadMe, endSession, can],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
