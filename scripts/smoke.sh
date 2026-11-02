#!/usr/bin/env bash
# Smoke test ponta a ponta contra a API rodando (make up).
# Fase 1: saúde, onboarding, login, /me, branding+logo, refresh, usuários, permissões, auditoria.
# A partir da fase 3 cobre também produto → estoque → venda → financeiro.
set -euo pipefail

API_URL="${API_URL:-http://localhost:8000}"
B="$API_URL/api/v1"
RUN_ID="$(date +%s)-$RANDOM"
OWNER_EMAIL="smoke-owner-$RUN_ID@example.com"
MEMBER_EMAIL="smoke-user-$RUN_ID@example.com"
PASS="smoke-pass-123"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT

step() { printf '\n\033[36m▶ %s\033[0m\n' "$*"; }
ok()   { printf '  \033[32m✔\033[0m %s\n' "$*"; }
fail() { printf '  \033[31m✘ %s\033[0m\n' "$*"; exit 1; }

# json <arquivo> <caminho.com.pontos>  (sem dependência de jq)
json() { python3 -c '
import json, sys
d = json.load(open(sys.argv[1]))
for k in sys.argv[2].split("."):
    d = d[int(k)] if isinstance(d, list) else d[k]
print(d if not isinstance(d, (dict, list)) else json.dumps(d))' "$1" "$2"; }

# req <método> <path> <status esperado> [args curl...]; corpo em $TMP/out
req() {
  local method=$1 path=$2 expected=$3; shift 3
  local code
  code=$(curl -sS -o "$TMP/out" -w '%{http_code}' -X "$method" "$B$path" "$@") || fail "$method $path: conexão falhou"
  [[ "$code" == "$expected" ]] || { cat "$TMP/out"; echo; fail "$method $path → $code (esperado $expected)"; }
  ok "$method $path → $code"
}
H() { echo "Authorization: Bearer $1"; }
JSONH=(-H 'Content-Type: application/json')

step "Saúde"
code=$(curl -s -o "$TMP/out" -w '%{http_code}' "$API_URL/health") || fail "API inacessível em $API_URL"
[[ "$code" == 200 ]] || { cat "$TMP/out"; fail "/health → $code"; }
ok "/health → $(cat "$TMP/out")"

step "Onboarding"
req POST /onboarding 201 "${JSONH[@]}" -d "{\"company\":{\"name\":\"Smoke $RUN_ID\",\"cnpj\":\"11.222.333/0001-81\"},\"owner\":{\"name\":\"Smoke Owner\",\"email\":\"$OWNER_EMAIL\",\"password\":\"$PASS\"}}"
req POST /onboarding 409 "${JSONH[@]}" -d "{\"company\":{\"name\":\"Dup\"},\"owner\":{\"name\":\"Dup Owner\",\"email\":\"$OWNER_EMAIL\",\"password\":\"$PASS\"}}"

step "Login e /me"
req POST /auth/login 401 "${JSONH[@]}" -d "{\"email\":\"$OWNER_EMAIL\",\"password\":\"errada\"}"
req POST /auth/login 200 "${JSONH[@]}" -d "{\"email\":\"$OWNER_EMAIL\",\"password\":\"$PASS\"}"
ACCESS=$(json "$TMP/out" access_token); REFRESH=$(json "$TMP/out" refresh_token)
req GET /me 200 -H "$(H "$ACCESS")"
[[ "$(json "$TMP/out" user.role)" == owner ]] || fail "papel deveria ser owner"
TENANT_ID=$(json "$TMP/out" tenant.id); ok "tenant $TENANT_ID"

step "Branding e logo"
req PUT /tenant/branding 200 "${JSONH[@]}" -H "$(H "$ACCESS")" -d '{"primary_color":"#7c3aed","secondary_color":"#1e1b4b"}'
[[ "$(json "$TMP/out" primary_color)" == "#7c3aed" ]] || fail "cor não atualizada"
python3 -c 'import sys,struct,zlib
def c(t,d): return struct.pack(">I",len(d))+t+d+struct.pack(">I",zlib.crc32(t+d))
raw=b"".join(b"\x00"+b"\x7c\x3a\xed"*8 for _ in range(8))
sys.stdout.buffer.write(b"\x89PNG\r\n\x1a\n"+c(b"IHDR",struct.pack(">IIBBBBB",8,8,8,2,0,0,0))+c(b"IDAT",zlib.compress(raw))+c(b"IEND",b""))' > "$TMP/logo.png"
req POST /tenant/branding/logo 200 -H "$(H "$ACCESS")" -F "file=@$TMP/logo.png;type=image/png"
LOGO_URL=$(json "$TMP/out" logo_url)
code=$(curl -s -o "$TMP/logo-down.png" -w '%{http_code}' "$LOGO_URL")
[[ "$code" == 200 ]] && cmp -s "$TMP/logo.png" "$TMP/logo-down.png" || fail "logo não acessível pela URL pré-assinada ($code)"
ok "logo baixado pela URL pré-assinada"

step "Refresh com rotação"
req POST /auth/refresh 200 "${JSONH[@]}" -d "{\"refresh_token\":\"$REFRESH\"}"
ACCESS=$(json "$TMP/out" access_token)
req POST /auth/refresh 401 "${JSONH[@]}" -d "{\"refresh_token\":\"$REFRESH\"}"
# reuso revogou tudo: login de novo
req POST /auth/login 200 "${JSONH[@]}" -d "{\"email\":\"$OWNER_EMAIL\",\"password\":\"$PASS\"}"
ACCESS=$(json "$TMP/out" access_token)

step "Usuários e permissões"
req POST /users 201 "${JSONH[@]}" -H "$(H "$ACCESS")" -d "{\"name\":\"Smoke User\",\"email\":\"$MEMBER_EMAIL\",\"password\":\"$PASS\",\"role\":\"user\"}"
MEMBER_ID=$(json "$TMP/out" id)
req POST /auth/login 200 "${JSONH[@]}" -d "{\"email\":\"$MEMBER_EMAIL\",\"password\":\"$PASS\"}"
MEMBER_ACCESS=$(json "$TMP/out" access_token)
req GET /users 403 -H "$(H "$MEMBER_ACCESS")"
req PUT /tenant/branding 403 "${JSONH[@]}" -H "$(H "$MEMBER_ACCESS")" -d '{"primary_color":"#000000"}'
req GET /tenant/branding 200 -H "$(H "$MEMBER_ACCESS")"
req PATCH "/users/$MEMBER_ID" 200 "${JSONH[@]}" -H "$(H "$ACCESS")" -d '{"is_active":false}'
req GET /me 401 -H "$(H "$MEMBER_ACCESS")"

step "Auditoria"
req GET "/audit?limit=100" 200 -H "$(H "$ACCESS")"
for action in tenant.create user.create auth.login branding.update branding.logo_upload user.deactivate; do
  grep -q "\"$action\"" "$TMP/out" || fail "auditoria sem $action"
done
ok "auditoria registrou onboarding, login, branding e usuários"

printf '\n\033[32mSMOKE OK\033[0m\n'
