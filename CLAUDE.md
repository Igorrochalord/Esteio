# CLAUDE.md: guia para agentes neste repositório

## Projeto
nucleo-erp: ERP modular SaaS. API FastAPI (monolito modular) + Postgres 16 com RLS + S3,
e cliente desktop Tauri v2/React. Decisões em `docs/adr/0001-arquitetura.md`.
**Não mude a arquitetura sem perguntar ao usuário.**

## Fluxo de trabalho
- Desenvolvido em fases. Ao fim de cada fase: `make reset && make up`, `make lint`,
  `make test`, `make smoke` (todos verdes), depois atualizar este arquivo e parar
  esperando "pode seguir".
- Commits pequenos em Conventional Commits (`feat(módulo): …`, `fix`, `test`, `build`, `docs`).
- Tudo roda em containers: não depende de Python/Node locais. Node local é 18; o
  frontend usa a imagem `node:22-alpine` pelos alvos `make web-*`.

## Regras do backend
- Módulo novo: `app/modules/<m>/{models,schemas,repository,service,router}.py`;
  registrar os modelos em `app/modules/models.py` e o router em `app/main.py`.
- Regra de negócio só em `service.py`. Router: valida (Pydantic) + `Depends(require(Module.X, Action.Y))` + delega.
- Services lançam `app.core.errors.*` (nunca `HTTPException`) e fazem `await ctx.db.commit()`
  no fim. Depois do commit o `app.tenant_id` some (é `is_local`); não consulte mais depois.
- Toda tabela de negócio: coluna `tenant_id` + `enable_tenant_rls(tabela)` + `grant_app(tabela)`
  na migração (`app/core/rls.py`). Migrações escritas à mão em `alembic/versions/NNNN_*.py`.
- Escrita relevante → `audit.record(...)` com `audit.snapshot()` / `audit.diff()`.
- Modelos com `updated_at`: usar `TimestampMixin` (eager_defaults; evita lazy-load async).
- Dinheiro: `Decimal` / `Numeric(14, 2)`. Nunca float.
- Novo módulo de permissão: adicionar em `Module` e na `MATRIX` de `app/core/permissions.py`.
- Testes: `tests/` com `client`, `onboard()`, `create_member()` do `conftest.py`. Todo
  módulo novo ganha teste de isolamento entre tenants (API e, se fizer sentido, SQL direto).

## Regras do frontend (apps/desktop)
- `src/services/api.ts`: `api<T>(path, opts)` com refresh automático (single-flight);
  endpoints tipados em `services/endpoints.ts`, tipos em `services/types.ts`.
- Autenticação via `useAuth()` (`me`, `can(module, level)`); rotas protegidas com
  `RequireAuth` / `RequirePermission` em `App.tsx` (HashRouter).
- Tema: `applyBranding()` define `--brand`, `--brand-ink`, `--brand-deep`, `--brand-deep-ink`.
  Use essas variáveis; não fixe cores de marca no CSS.
- Textos em pt-BR, voltados ao usuário final (sem jargão técnico).

## Ambiente
- `make up` / `reset` / `test` / `lint` / `smoke` / `seed` / `web-dev` (ver README).
- S3 local é SeaweedFS (MinIO saiu do Docker Hub), bucket criado por `storage-init`.
- Usuários do seed: senha `nucleo123` (dono@paodourado.com.br, dono@techparts.com.br, …).

## Estado
- **Fase 1 (fundação): concluída.** tenants, auth (login/refresh/logout/me), users,
  branding + logo, auditoria, RLS, testes de isolamento, seed, smoke, desktop
  (onboarding, login, layout, pessoas, empresa, histórico).
- Próxima: Fase 2: estoque (categorias, fornecedores, produtos, movimentações imutáveis,
  alertas de mínimo).
- Pendências conhecidas: refresh token do desktop em `localStorage` (mover para o cofre
  do SO antes de distribuir); `npm run tauri build` ainda não executado (sem Rust no
  ambiente; previsto no CI da fase 7); smoke da fase 1 não cobre venda/estoque (entra na fase 3).
