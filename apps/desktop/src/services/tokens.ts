import { readKey, writeKey } from "./storage";

// access token só em memória; refresh persistido para manter a sessão entre aberturas.
// TODO(segurança): no app Tauri, mover o refresh para armazenamento seguro do SO.
const REFRESH_KEY = "nucleo.refresh";
let accessToken: string | null = null;

export const tokens = {
  get access() {
    return accessToken;
  },
  get refresh() {
    return readKey(REFRESH_KEY);
  },
  set(access: string, refresh: string) {
    accessToken = access;
    writeKey(REFRESH_KEY, refresh);
  },
  clear() {
    accessToken = null;
    writeKey(REFRESH_KEY, null);
  },
};
