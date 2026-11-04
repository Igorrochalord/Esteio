import { invoke } from "@tauri-apps/api/core";

/** Erro devolvido pelo núcleo (`CoreError`): `message` já vem pronta para o usuário. */
export class ApiError extends Error {
  readonly code: string;
  readonly fields: Record<string, string>;

  constructor(message: string, code = "internal", fields: Record<string, string> = {}) {
    super(message);
    this.code = code;
    this.fields = fields;
  }
}

type Listener = () => void;
const sessionExpiredListeners = new Set<Listener>();

/** Avisado quando a sessão termina (usuário desativado, backup restaurado…). */
export function onSessionExpired(fn: Listener): () => void {
  sessionExpiredListeners.add(fn);
  return () => sessionExpiredListeners.delete(fn);
}

interface CoreErrorPayload {
  code?: string;
  message?: string;
  fields?: Record<string, string>;
}

function toError(raw: unknown): ApiError {
  if (raw && typeof raw === "object" && "message" in raw) {
    const e = raw as CoreErrorPayload;
    return new ApiError(e.message ?? "Ocorreu um erro inesperado.", e.code, e.fields ?? {});
  }
  if (typeof raw === "string") return new ApiError(raw);
  return new ApiError("Ocorreu um erro inesperado. Tente novamente.");
}

/** Chama um comando do núcleo (Rust). Não há rede: tudo roda neste computador. */
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (raw) {
    const err = toError(raw);
    if (err.code === "unauthorized" && command !== "login") {
      sessionExpiredListeners.forEach((fn) => fn());
    }
    throw err;
  }
}
