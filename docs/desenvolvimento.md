# Desenvolvimento

## Pré-requisitos

| Ferramenta | Para quê | Instalação |
|---|---|---|
| Rust 1.91+ | núcleo e app | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node 18+ (22 recomendado) | interface e CLI do Tauri | gerenciador do SO / nvm |
| Libs do WebKit (Linux/WSL) | compilar e abrir o app | ver abaixo |
| `make` | atalhos | já vem no Linux |

Linux / WSL (Ubuntu 24.04):

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
  libxdo-dev libssl-dev libdbus-1-dev pkg-config build-essential
```

No WSL, a janela abre pelo WSLg. No Windows nativo, basta instalar o Rust (rustup), o
*Visual Studio Build Tools* (C++) e o Node; o WebView2 já vem no Windows 10/11.

Os testes do núcleo (`make test`) só precisam do Rust. As libs do WebKit são exigidas para
`make lint`, `make dev` e `make build`, que compilam o app.

## Comandos

| Comando | O que faz |
|---|---|
| `make install` | `npm ci` do frontend |
| `make test` | testes do núcleo (`args=nome_do_teste` para filtrar) |
| `make lint` | `cargo fmt --check`, `clippy -D warnings`, `tsc`, `eslint` |
| `make fmt` | formata o Rust |
| `make check` | `lint` + `test`: rodar antes de cada commit |
| `make seed` | cria empresa de demonstração em `.dev-data/` (`empresa=techparts` para a outra) |
| `make reset-data` | apaga `.dev-data/` |
| `make dev` | abre o app usando `.dev-data/` (`NUCLEO_DATA_DIR`) |
| `make build` | gera o instalador da plataforma atual ([distribuição](distribuicao.md)) |

## Testes

- Ficam em `crates/nucleo-core/tests/`. Cada teste abre um banco novo numa pasta temporária
  (`common::setup()` e `common::onboarded()`).
- Helpers: `onboarded()` (empresa + dono logado), `create_member(core, email, papel)` e
  `login_as(core, email)`.
- Todo módulo novo tem testes de: regras de negócio, permissão por papel e auditoria.
- `tests/foundation.rs` falha se uma tabela de negócio nova não seguir as regras de sync
  (colunas, triggers de `change_log` e bloqueio de DELETE).

## Criando um módulo (ex.: estoque)

1. **Migração** `crates/nucleo-core/migrations/NNNN_<nome>.sql` com as
   [regras de tabela](banco-de-dados.md#regras-para-toda-tabela-de-negócio). Nunca edite uma migração já publicada.
2. **Código** em `crates/nucleo-core/src/modules/<m>/{models,repository,service}.rs` e
   registro em `modules/mod.rs`.
3. **Permissão**: adicione o módulo em `Module` (`ALL`, `as_str`) e em `granted()` de
   `src/permissions.rs`; no front, em `Module` de `services/types.ts`.
4. **Comandos** em `apps/desktop/src-tauri/src/commands/<m>.rs`, registrados no
   `generate_handler!` de `src-tauri/src/lib.rs`.
5. **Frontend**: tipos em `services/types.ts`, métodos em `services/endpoints.ts`, página
   em `pages/`, rota em `App.tsx` e item de menu em `layouts/AppLayout.tsx`.
6. **Testes** em `crates/nucleo-core/tests/<m>.rs`.

## Regras de código

- Regra de negócio **só no service**. O comando apenas chama `core.require(...)` e delega.
- Services devolvem `CoreError` (`NotFound`, `Conflict`, `Forbidden`, `Unauthorized`,
  `BusinessRule`, `Validation`) com mensagem em pt-BR para o usuário final.
- Escrita: `ctx.db.begin()` → alterações → `audit::record`/`record_changes` → `tx.commit()`.
- Dinheiro: `INTEGER` em centavos no banco e `rust_decimal` no domínio. Nunca float.
- Senhas e bytes grandes nunca vão para a auditoria: use `#[serde(skip_serializing)]` no model.
- Frontend: textos em pt-BR, sem jargão técnico. Cores da marca só via variáveis `--brand*`.
- Commits pequenos em Conventional Commits: `feat(módulo): …`, `fix`, `test`, `build`, `docs`.
