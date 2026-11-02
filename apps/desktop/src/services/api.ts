import { getApiUrl } from "./config";
import { tokens } from "./tokens";
import type { TokenPair } from "./types";

export class ApiError extends Error {
  readonly status: number;
  readonly code?: string;
  readonly fields: Record<string, string>;

  constructor(status: number, message: string, code?: string, fields: Record<string, string> = {}) {
    super(message);
    this.status = status;
    this.code = code;
    this.fields = fields;
  }
}

interface RequestOptions {
  method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  body?: unknown;
  form?: FormData;
  /** false = não envia token nem tenta refresh (login, onboarding) */
  auth?: boolean;
}

type Listener = () => void;
const sessionExpiredListeners = new Set<Listener>();

/** Avisado quando o refresh falha e o usuário precisa entrar de novo. */
export function onSessionExpired(fn: Listener): () => void {
  sessionExpiredListeners.add(fn);
  return () => sessionExpiredListeners.delete(fn);
}

async function send(path: string, opts: RequestOptions): Promise<Response> {
  const headers: Record<string, string> = { Accept: "application/json" };
  let body: BodyInit | undefined;
  if (opts.form) {
    body = opts.form;
  } else if (opts.body !== undefined) {
    headers["Content-Type"] = "application/json";
    body = JSON.stringify(opts.body);
  }
  if (opts.auth !== false && tokens.access) {
    headers.Authorization = `Bearer ${tokens.access}`;
  }
  try {
    return await fetch(`${getApiUrl()}/api/v1${path}`, {
      method: opts.method ?? "GET",
      headers,
      body,
    });
  } catch {
    throw new ApiError(0, `Sem conexão com o servidor (${getApiUrl()}).`);
  }
}

// um único refresh em voo: várias requisições com 401 aguardam o mesmo
let refreshing: Promise<boolean> | null = null;

export function refreshSession(): Promise<boolean> {
  if (!refreshing) {
    refreshing = (async () => {
      const refresh = tokens.refresh;
      if (!refresh) return false;
      try {
        const res = await send("/auth/refresh", {
          method: "POST",
          body: { refresh_token: refresh },
          auth: false,
        });
        if (!res.ok) {
          tokens.clear();
          return false;
        }
        const pair = (await res.json()) as TokenPair;
        tokens.set(pair.access_token, pair.refresh_token);
        return true;
      } catch {
        return false; // sem rede: mantém o refresh para tentar depois
      }
    })().finally(() => {
      refreshing = null;
    });
  }
  return refreshing;
}

async function toError(res: Response): Promise<ApiError> {
  let data: { detail?: unknown; code?: string } = {};
  try {
    data = await res.json();
  } catch {
    /* corpo não-JSON */
  }
  // FastAPI: erros de validação vêm como lista em detail
  if (Array.isArray(data.detail)) {
    const fields: Record<string, string> = {};
    for (const item of data.detail as { loc?: unknown[]; msg?: string }[]) {
      const field = (item.loc ?? []).filter((p) => typeof p === "string" && p !== "body").join(".");
      fields[field || "_"] = (item.msg ?? "inválido").replace(/^Value error, /, "");
    }
    return new ApiError(res.status, "Confira os campos destacados.", "validation", fields);
  }
  const message =
    typeof data.detail === "string" ? data.detail : `Erro inesperado do servidor (${res.status}).`;
  return new ApiError(res.status, message, data.code);
}

export async function api<T>(path: string, opts: RequestOptions = {}): Promise<T> {
  let res = await send(path, opts);
  if (res.status === 401 && opts.auth !== false && tokens.refresh) {
    if (await refreshSession()) {
      res = await send(path, opts);
    } else {
      tokens.clear();
      sessionExpiredListeners.forEach((fn) => fn());
    }
  }
  if (!res.ok) throw await toError(res);
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}
