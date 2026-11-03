# ADR 0001: Arquitetura base

- Status: substituída parcialmente pelo [ADR 0002](0002-executavel-local.md)
- Data: 2026-11-02

## Contexto

ERP modular vendido como SaaS para pequenas e médias empresas, com cliente desktop.
Precisamos de isolamento forte entre clientes, custo de operação baixo e uma base
simples para um time pequeno evoluir por módulos.

## Decisões

1. **Monolito modular em FastAPI.** Cada módulo em `app/modules/<nome>` com
   `models / schemas / repository / service / router`. A regra de negócio fica em
   `service.py`; o router só valida e delega. Services lançam erros de domínio
   (`app/core/errors.py`), que o `main.py` traduz para HTTP.

2. **Multi-tenant em banco compartilhado com RLS.**
   - Toda tabela de negócio tem `tenant_id`, com `ENABLE` + `FORCE ROW LEVEL SECURITY` e a
     policy `tenant_isolation` (`tenant_id = app_current_tenant()`), usando
     `app.core.rls.enable_tenant_rls()` nas migrações.
   - Por requisição: `set_config('app.tenant_id', <tid do token>, true)`, equivalente
     parametrizado de `SET LOCAL`. Vale só até o fim da transação.
   - `app_current_tenant()` devolve NULL quando não há tenant, e então nenhuma linha
     fica visível. Falha fechada.
   - Três papéis no Postgres: `nucleo_migrator` (dono do schema, roda o Alembic),
     `nucleo_app` (API, `NOBYPASSRLS`, só DML) e `nucleo_auth` (`NOLOGIN BYPASSRLS`).
     O `nucleo_auth` é dono apenas da função `auth_find_user(email)`.

3. **Login cross-tenant.** O e-mail é único no sistema todo (índice em `lower(email)`).
   No login ainda não sabemos o tenant, então `auth_find_user()` (`SECURITY DEFINER`)
   devolve só `id, tenant_id, password_hash, is_active`. É o único ponto que contorna
   a RLS.

4. **Transações explícitas.** O service que altera dados faz `commit()` no fim da
   unidade de trabalho; se não fizer, a sessão é fechada e tudo volta (rollback).
   Não dependemos do momento em que o FastAPI encerra dependências com `yield`.

5. **Auth.** O access token é um JWT curto (15 min). O refresh token é um JWT com `jti`
   registrado em `refresh_tokens`, com rotação a cada uso. Reusar um refresh revogado
   revoga todas as sessões do usuário. Senhas usam argon2. Desativar um usuário corta
   o acesso na hora, porque o usuário é relido do banco a cada requisição.

6. **RBAC.** Matriz estática papel → módulo → nível (`read < write < manage`) em
   `app/core/permissions.py`. `/me` devolve a matriz efetiva para o frontend montar
   o menu.

7. **Auditoria append-only.** `audit_logs` (quem, quando, ação, entidade, antes/depois,
   IP). A aplicação tem só `INSERT/SELECT`. O before/after guarda apenas campos
   alterados e nunca `password_hash`.

8. **Arquivos.** O banco guarda só a chave do objeto. O cliente recebe URLs
   pré-assinadas geradas com `S3_PUBLIC_ENDPOINT_URL`. Logos aceitam PNG/JPEG
   (verificação por assinatura de bytes); SVG é recusado por poder conter script.

9. **S3 em dev: SeaweedFS.** As imagens oficiais do MinIO deixaram de ser distribuídas
   publicamente (pull falha em Docker Hub e Quay). Usamos `chrislusf/seaweedfs`
   (Apache 2.0) no modo `server -s3`. O código depende só da API S3 via boto3, então
   produção usa AWS S3 sem mudança.

10. **Dinheiro** sempre `Decimal` / `NUMERIC(14,2)`; nunca float. Começa a valer na fase 2.

11. **Desktop só como interface.** O Tauri v2 empacota o build do Vite. Usamos
    HashRouter porque não há servidor para fallback de rotas. A URL da API é
    configurável em tempo de execução. O access token fica em memória e o refresh em
    `localStorage`; antes de distribuir, mover o refresh para o cofre do SO.

## Consequências

- Esquecer a RLS em uma tabela nova é o maior risco. Mitigação: helper obrigatório
  nas migrações e testes de isolamento por módulo.
- O e-mail global impede a mesma pessoa de ter contas em duas empresas com o mesmo
  e-mail. Aceito por ora; dá para evoluir para "usuário × vínculos" se precisar.
- Pontos de extensão fora do escopo atual (NF-e, cobrança, IA, auto-update): novos
  módulos em `app/modules/`, sem mudança na fundação.
