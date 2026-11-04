# CLAUDE.md: guia para agentes neste repositório

## Projeto
Núcleo ERP: ERP modular **instalável** (Tauri v2 + React na interface, Rust + SQLite no núcleo).
Sem servidor, sem HTTP e com uma empresa por instalação. Decisão em `docs/adr/0002-executavel-local.md`.
**Não mude a arquitetura sem perguntar ao usuário.** A documentação por tópico está em `docs/`
(estrutura, arquitetura, desenvolvimento, banco-de-dados, seguranca-e-permissoes, backup, distribuicao,
assinatura, roteiro).

## Fluxo de trabalho
- Desenvolvido em fases (`docs/roteiro.md`). Ao fim de cada fase: `make check` verde, atualizar
  `docs/` e este arquivo, depois parar e esperar "pode seguir".
- Commits pequenos em Conventional Commits (`feat(módulo): …`, `fix`, `test`, `build`, `docs`).
- Ferramentas locais: Rust (`~/.cargo/bin`, 1.91) e Node 18+. Não usa Docker.
  `make test` só precisa do Rust; `lint`/`dev`/`build` precisam das libs do WebKit.

## Regras do núcleo (`crates/nucleo-core`)
- Módulo novo: `src/modules/<m>/{models,repository,service}.rs` + `modules/mod.rs`; comandos em
  `apps/desktop/src-tauri/src/commands/<m>.rs`, registrados no `generate_handler!` de `src-tauri/src/lib.rs`.
- Regra de negócio só no service. O comando faz `core.require(Module::X, Action::Y)` e delega.
- Services devolvem `CoreError` (mensagem pt-BR para o usuário final), validam com `Validator`
  (chave = caminho do campo, ex.: `owner.email`) e fazem `tx.commit()` no fim.
- Toda tabela de negócio: id UUID v7 (`db::new_id`), `branch_id`, `device_id`, `created_at`,
  `updated_at`, `deleted_at`, mais os triggers `_cl_insert`, `_cl_update` e `_no_delete`
  (`tests/foundation.rs` cobra isso). Migrações SQL à mão em `migrations/NNNN_*.sql`; nunca editar uma publicada.
- Escrita relevante → `audit::record` ou `audit::record_changes`. Campos sensíveis levam `#[serde(skip_serializing)]`.
- Dinheiro: `INTEGER` em centavos no banco e `rust_decimal` no domínio. Nunca float.
- Novo módulo de permissão: `Module` (+`ALL`, `as_str`) e `granted()` em `src/permissions.rs`;
  no front, `Module` em `services/types.ts`.
- Testes em `tests/` com `common::{onboarded, create_member, login_as}`: regras, permissão por papel e auditoria.

## Regras do frontend (`apps/desktop/src`)
- `services/api.ts`: `call<T>(comando, args)` sobre `invoke` e `ApiError {code, message, fields}`.
- Listas usam `components/DataGrid.tsx` (edição na célula, ordenação, totais, exportar .xlsx).
  Comandos tipados em `services/endpoints.ts`, tipos em `services/types.ts`. Argumentos em snake_case.
- `useAuth()` (`status`: loading/setup/anonymous/authenticated, `me`, `can`); rotas com
  `RequireAuth` / `RequirePermission` em `App.tsx` (HashRouter).
- Tema: `applyBranding()` define `--brand`, `--brand-ink`, `--brand-deep` e `--brand-deep-ink`. Não fixe cores de marca.
- Textos em pt-BR, voltados ao usuário final (sem jargão técnico).

## Ambiente
- `make install` / `test` / `lint` / `check` / `seed` / `reset-data` / `dev` / `build`.
- Dados de dev em `.dev-data/` (`NUCLEO_DATA_DIR`). Seed: senha `nucleo123` (dono@paodourado.com.br, …).
- Instalador Windows: CI (`.github/workflows/ci.yml`, runner windows-latest).

## Estado
- **Fase 1b (fundação local)**: núcleo, comandos, frontend e CI prontos; `make check` verde e o app
  abre no WSL (`make dev`).
- Pendente: roteiro manual completo na interface; primeiro instalador pelo CI (requer push).
- **Fase 2 (comercial)**: produtos/estoque, clientes, vendas, relatórios, grade tipo planilha e
  Excel prontos e testados; aguardando validação do usuário na interface.
- Dinheiro em centavos e quantidades em milésimos no banco; `Decimal` (texto) na API (`src/money.rs`).
- Gráficos: seguir a skill dataviz (série única, rampa sequencial validada, tabela junto).
- O código SaaS antigo (FastAPI/Postgres) está na tag `v0.1-saas`.
