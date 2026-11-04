# Arquitetura

Decisão completa em [ADR 0002](adr/0002-executavel-local.md).

## Visão geral

```
nucleo-erp.exe (Tauri v2)
 ├─ interface React ──── invoke("comando", args)      ← sem HTTP, tudo no mesmo processo
 ├─ src-tauri/commands   checa sessão e permissão, delega
 └─ nucleo-core          regras de negócio
      └─ SQLite          <pasta de dados>/nucleo.db
```

- **Um programa, uma empresa.** Cada instalação tem uma única empresa, gravada na tabela `company`.
- **Sem servidor.** Não existe API, porta aberta ou acesso à internet. A CSP do app bloqueia rede.
- **Pronto para filiais.** As tabelas já guardam o que um sync futuro vai precisar
  (ver [banco de dados](banco-de-dados.md#preparado-para-sincronizar)).

## Camadas

| Camada | Onde | Responsabilidade |
|---|---|---|
| Interface | `apps/desktop/src` | telas, formulários, texto para o usuário |
| Serviços do front | `src/services/{api,endpoints,types}.ts` | `call()` sobre `invoke`, um método por comando, tipos |
| Comandos | `src-tauri/src/commands/<m>.rs` | `core.require(Módulo, Ação)` e delega; nenhuma regra aqui |
| Service | `nucleo-core/src/modules/<m>/service.rs` | validação, regras, transação, auditoria, `commit` |
| Repository | `.../<m>/repository.rs` | SQL do módulo |
| Models | `.../<m>/models.rs` | structs do banco e de entrada/saída (serde) |

## Caminho de uma ação

Exemplo: o dono troca a cor principal.

1. `CompanyPage` chama `companyApi.updateBranding({ primary_color })`.
2. `call("branding_update", { data })` vira `invoke` do Tauri.
3. `commands::company::branding_update` chama `core.require(Settings, Manage)`, que relê o
   usuário da sessão no banco e confere a matriz de permissões.
4. `company::service::update_branding` valida a cor, abre a transação, altera, grava a
   auditoria só com os campos que mudaram e faz `commit`. O trigger registra a mudança no `change_log`.
5. Um erro volta como `CoreError` `{ code, message, fields }`, e a tela mostra `message`, que já vem em pt-BR.

## Estado em memória (`Core`)

`nucleo_core::Core` é criado na abertura do app (`src-tauri/src/lib.rs`) e guarda:

- o pool do SQLite (trocado na restauração de backup);
- a identidade da instalação (`device_id`, `branch_id`);
- a sessão: o id do usuário logado, **só em memória**, que some ao fechar o app;
- uma trava de manutenção, que serializa backup e restauração.

## Módulos atuais

| Módulo | Comandos | Permissão |
|---|---|---|
| auth | `setup_status`, `login_branding`, `register`, `onboard`, `login`, `logout`, `me` | sem login (exceto `me`) |
| company | `company_get`, `company_update`, `branding_update`, `logo_upload`, `logo_remove` | settings |
| users | `users_list`, `users_create`, `users_update`, `users_approve`, `users_reject`, `users_reset_password` | users |
| audit | `audit_list` | audit |
| backup | `backup_list`, `backup_create`, `backup_restore` | backup |
| products | `products_list/create/update`, `stock_move`, `stock_movements`, `categories_*` | products |
| customers | `customers_list/create/update` | customers |
| sales | `sales_list/get/create`, `sales_cancel` (manage), `sellers_list` | sales |
| reports | `report_sales`, `report_products`, `report_stock`, `report_customers`, `report_sellers` | reports |
| spreadsheet | `export_xlsx` (qualquer logado), `import_preview`, `import_products` (products write) | — |

## "Excel interno": a grade

`components/DataGrid.tsx` é usado em todas as listas: setas movem a célula ativa; Enter, F2,
duplo clique ou começar a digitar editam; Enter grava e desce, Tab vai para o lado, Esc cancela;
Ctrl+C copia; clique no cabeçalho ordena; busca filtra todas as colunas; linha de totais;
"Exportar Excel" grava exatamente o que está visível (filtrado e ordenado). Cada célula
editada chama o comando `*_update` do módulo com **só aquele campo**.

## Tutorial e planos na interface

- `components/Tour.tsx`: tutorial em tela cheia com passos por papel (proprietário,
  administrador, usuário). Abre sozinho no primeiro acesso de cada pessoa neste computador
  (`localStorage nucleo.tour.v1.<id>`) e pelo botão **Como usar** no menu.
- `pages/PlansPage.tsx` (`/planos`, para proprietário e administrador): planos, simulador,
  comparativo. Preços e limites vêm de `plans_catalog` (`core/src/plans.rs`); hoje tudo liberado.
- `components/PlansNudge.tsx`: lembrete no canto a cada `PLAN_NUDGE_INTERVAL_MS` (5 min), só
  para quem pode assinar, nunca sobre um diálogo aberto nem na própria página de planos.

## Gráficos

`components/charts.tsx` (SVG próprio): `ColumnChart`, `BarList` (ranking), `StatTile` e
`BrazilTileMap` (mapa em blocos por UF). Uma série por gráfico, rampa sequencial azul
validada (`validate_palette --ordinal`), tooltip no hover e no foco do teclado, e cada gráfico
tem a tabela correspondente ao lado. Datas dos relatórios agrupadas no **fuso local**
(`tz_offset_minutes` enviado pela tela).

## Frontend

- `HashRouter`: o app carrega arquivos locais, então não há servidor para resolver as rotas.
- `useAuth()` expõe `status` (`loading` · `setup` · `anonymous` · `authenticated`), `me` e
  `can(módulo, nível)`. As rotas são protegidas por `RequireAuth` e `RequirePermission` em `App.tsx`.
- Primeira abertura (`setup`): só o onboarding fica acessível. Depois: login e "Pedir cadastro"
  (`/cadastro`), ambos dentro do `AuthFrame`, que já mostra nome, logo e cores da empresa.
- Tema: `applyBranding()` define `--brand`, `--brand-ink`, `--brand-deep` e `--brand-deep-ink`.
