import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";

import { AuthContext, type AuthState, type AuthStatus } from "../hooks/authContext";
import { onSessionExpired, refreshSession } from "../services/api";
import { authApi } from "../services/endpoints";
import { applyBranding } from "../services/theme";
import { tokens } from "../services/tokens";
import type { Level, Me, Module, TokenPair } from "../services/types";

const RANK: Record<Level, number> = { read: 1, write: 2, manage: 3 };

export function AuthProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<AuthStatus>("loading");
  const [me, setMe] = useState<Me | null>(null);

  const signOutLocally = useCallback(() => {
    tokens.clear();
    setMe(null);
    setStatus("anonymous");
    applyBranding(null);
  }, []);

  const loadMe = useCallback(async () => {
    const data = await authApi.me();
    setMe(data);
    setStatus("authenticated");
    applyBranding(data.tenant);
  }, []);

  // ao abrir o app: tenta retomar a sessão com o refresh salvo
  useEffect(() => {
    (async () => {
      if (tokens.refresh && (await refreshSession())) {
        try {
          await loadMe();
          return;
        } catch {
          /* cai para anônimo */
        }
      }
      signOutLocally();
    })();
  }, [loadMe, signOutLocally]);

  useEffect(() => onSessionExpired(signOutLocally), [signOutLocally]);

  const startSession = useCallback(
    async (pair: TokenPair) => {
      tokens.set(pair.access_token, pair.refresh_token);
      await loadMe();
    },
    [loadMe],
  );

  const login = useCallback(
    async (email: string, password: string) => startSession(await authApi.login(email, password)),
    [startSession],
  );

  const logout = useCallback(async () => {
    const refresh = tokens.refresh;
    signOutLocally();
    if (refresh) await authApi.logout(refresh).catch(() => undefined);
  }, [signOutLocally]);

  const can = useCallback(
    (module: Module, level: Level = "read") => {
      const granted = me?.permissions[module];
      return !!granted && RANK[granted] >= RANK[level];
    },
    [me],
  );

  const value = useMemo<AuthState>(
    () => ({ status, me, login, startSession, logout, reloadMe: loadMe, can }),
    [status, me, login, startSession, logout, loadMe, can],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
