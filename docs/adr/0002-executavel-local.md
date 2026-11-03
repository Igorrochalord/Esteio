# ADR 0002: Executável local (Tauri + Rust + SQLite)

- Status: aceita
- Data: 2026-11-03
- Substitui: itens 1, 2, 3, 5, 8, 9 e 11 do [ADR 0001](0001-arquitetura.md)

## Contexto

O produto deve ser um **sistema instalável**. Cada empresa instala o `.exe` e os dados ficam
no próprio computador, sem depender de servidor, internet ou HTTP. O ADR 0001 descrevia um SaaS
(API FastAPI + Postgres com RLS + S3) com o desktop como mera interface. As empresas-alvo têm
filiais, então no futuro os dados de lojas diferentes vão precisar se encontrar.

A fundação (fase 1) tinha ~2.200 linhas de backend quando a decisão foi tomada, então a
reescrita sai barata agora. O código SaaS fica preservado na tag `v0.1-saas`.

## Decisões

1. **Tudo dentro do executável.** O Tauri v2 empacota a interface React e as regras de negócio
   em Rust. O React chama `invoke("comando", args)`; não há HTTP nem servidor.

2. **Camadas.** O crate `crates/nucleo-core` guarda a regra de negócio pura e testável sem GUI:
   `modules/<m>/{models,repository,service}.rs`, `errors`, `permissions`, `audit`, `db`.
   O `apps/desktop/src-tauri/src/commands/<m>.rs` é a camada fina (equivale ao router): valida,
   checa permissão e delega. O service lança `CoreError` e faz commit da transação.

3. **SQLite local** via `sqlx`, com migrações SQL em `crates/nucleo-core/migrations`, aplicadas
   ao abrir o app. PRAGMAs: `journal_mode=WAL`, `foreign_keys=ON`, `busy_timeout`. O arquivo fica
   no diretório de dados do app (`app_data_dir`).

4. **Uma empresa por instalação.** Saem `tenant_id`, RLS e o login cross-tenant. A tabela
   `company` tem uma única linha, com dados, branding e logo (BLOB), para que o backup seja um arquivo só.

5. **Pronto para sincronizar** (vale para toda tabela de negócio):
   - PK `TEXT` com UUID v7, nunca autoincremento;
   - `created_at`, `updated_at` e `deleted_at` (exclusão lógica), em UTC ISO-8601;
   - `branch_id` (filial) e `device_id` (instalação);
   - cada escrita registra uma linha em `change_log` (outbox) na mesma transação.
   O serviço de sincronização entre filiais é uma fase futura.

6. **Auth local.** Usuários com senha `argon2`. A sessão fica na memória do processo Rust,
   sem JWT nem refresh token. O usuário é relido do banco a cada comando, então desativar alguém corta o acesso na hora.

7. **Dinheiro**: `INTEGER` em centavos no banco e `rust_decimal` no domínio. Nunca float.

8. **Backup**: `VACUUM INTO`, com cópia automática diária (rotação de 7) e restauração pela interface.

9. **Distribuição**: o instalador Windows (NSIS/MSI) é gerado no CI (`windows-latest` +
   `tauri-action`). Antes de distribuir: assinatura de código e `tauri-plugin-updater`.

Continuam valendo do ADR 0001: monolito modular, RBAC por matriz estática, auditoria
append-only (reforçada por triggers no SQLite), transações explícitas e textos em pt-BR.

## Consequências

- Não há isolamento entre empresas para manter. O maior risco passa a ser a **perda do
  arquivo**, e a mitigação é o backup automático com restauração testada.
- Filiais só compartilham dados depois da fase de sincronização. Até lá, cada instalação é independente.
- Esquecer as colunas de sync ou o `change_log` numa tabela nova é o novo risco estrutural.
  Mitigação: helper no repository e teste que confere toda tabela de negócio.
- O desenvolvimento passa a exigir toolchain Rust (container no Linux/WSL, nativo no Windows).
