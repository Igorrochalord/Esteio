import { readKey, writeKey } from "./storage";

const API_URL_KEY = "nucleo.apiUrl";
const DEFAULT_API_URL = import.meta.env.VITE_API_URL || "http://localhost:8000";

/** URL da API: override salvo pelo usuário > VITE_API_URL > localhost. */
export function getApiUrl(): string {
  return (readKey(API_URL_KEY) || DEFAULT_API_URL).replace(/\/+$/, "");
}

export function setApiUrl(url: string | null): void {
  writeKey(API_URL_KEY, url ? url.trim().replace(/\/+$/, "") : null);
}

export function isDefaultApiUrl(): boolean {
  return !readKey(API_URL_KEY);
}
