#!/usr/bin/env node
// Grava a versão nos 3 arquivos do app (Cargo.toml do workspace, tauri.conf.json e package.json).
// Usado pelo `make version` e pelo CI na hora de gerar a release. Funciona em Linux e Windows.
//   node scripts/set-version.mjs 0.3.0
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const v = (process.argv[2] ?? "").replace(/^[vV]/, "");
if (!/^\d+\.\d+\.\d+$/.test(v)) {
  console.error("uso: node scripts/set-version.mjs 0.3.0");
  process.exit(1);
}
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const edit = (file, re, to) => {
  const path = join(root, file);
  const before = readFileSync(path, "utf8");
  const after = before.replace(re, to);
  if (after === before && !before.match(re)) throw new Error(`versão não encontrada em ${file}`);
  writeFileSync(path, after);
  console.log(`${file}: ${v}`);
};
edit("Cargo.toml", /^version = ".*"$/m, `version = "${v}"`);
edit("apps/desktop/src-tauri/tauri.conf.json", /"version": ".*?"/, `"version": "${v}"`);
edit("apps/desktop/package.json", /"version": ".*?"/, `"version": "${v}"`);
