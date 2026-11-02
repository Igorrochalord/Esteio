import { createContext } from "react";

import type { Level, Me, Module, TokenPair } from "../services/types";

export type AuthStatus = "loading" | "anonymous" | "authenticated";

export interface AuthState {
  status: AuthStatus;
  me: Me | null;
  login: (email: string, password: string) => Promise<void>;
  /** usado após o onboarding, que já devolve tokens */
  startSession: (pair: TokenPair) => Promise<void>;
  logout: () => Promise<void>;
  reloadMe: () => Promise<void>;
  can: (module: Module, level?: Level) => boolean;
}

export const AuthContext = createContext<AuthState | null>(null);
