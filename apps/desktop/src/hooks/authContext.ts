import { createContext } from "react";

import type { Level, Me, Module } from "../services/types";

/** `setup` = primeira abertura neste computador (ainda não há empresa). */
export type AuthStatus = "loading" | "setup" | "anonymous" | "authenticated";

export interface AuthState {
  status: AuthStatus;
  me: Me | null;
  login: (email: string, password: string) => Promise<void>;
  /** usado após o onboarding, que já entra como dono */
  startSession: (me: Me) => void;
  logout: () => Promise<void>;
  reloadMe: () => Promise<void>;
  /** sessão encerrada pelo sistema (ex.: backup restaurado) */
  endSession: () => void;
  can: (module: Module, level?: Level) => boolean;
}

export const AuthContext = createContext<AuthState | null>(null);
