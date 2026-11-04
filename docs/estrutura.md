# Estrutura de pastas

O que cada pasta faz e quando você mexe nela.

```
appIgor/
├─ .github/workflows/        automação no GitHub (CI)
├─ apps/desktop/             o programa que o cliente instala
│  ├─ public/                arquivos estáticos da interface
│  ├─ src/                   interface (React)
│  │  ├─ components/         peças reutilizáveis de tela
│  │  ├─ hooks/              lógica de React compartilhada
│  │  ├─ layouts/            molduras das páginas
│  │  ├─ pages/              uma tela por arquivo
│  │  └─ services/           conversa com o núcleo, tipos, textos, tema
│  └─ src-tauri/             a "casca" nativa do programa (Rust)
│     ├─ capabilities/       o que a interface tem permissão de fazer
│     ├─ icons/              ícones do programa e do instalador
│     └─ src/commands/       comandos que a interface pode chamar
├─ crates/nucleo-core/       regras de negócio e banco de dados (Rust)
│  ├─ migrations/            criação e evolução das tabelas
│  ├─ src/                   código do núcleo
│  │  ├─ bin/                programas auxiliares (seed)
│  │  └─ modules/            um módulo de negócio por pasta
│  └─ tests/                 testes automáticos
├─ docs/                     esta documentação
│  └─ adr/                   registro das decisões de arquitetura
└─ target/  .dev-data/       gerados localmente (fora do git)
```

## Raiz

| Arquivo | Para quê |
|---|---|
| `Cargo.toml` | workspace Rust: junta o núcleo e o app; versão do produto e perfis de compilação |
| `Cargo.lock` | versões exatas das bibliotecas Rust (commitado: o build é reproduzível) |
| `Makefile` | atalhos do dia a dia (`make test`, `make dev`, …) |
| `README.md` | porta de entrada do projeto |
| `CLAUDE.md` | regras para agentes de IA que trabalham no repositório |
| `.editorconfig` / `.gitignore` | padrão de formatação / o que não vai para o git |

## `.github/workflows/`

`ci.yml` roda a cada push. No Ubuntu, faz lint e testes; no Windows, gera o instalador. Numa tag
`v*`, cria o release para download. Detalhes em [distribuição](distribuicao.md).

## `apps/desktop/`: o programa

Tudo que vira o `.exe`. Contém a interface (`src/`) e a casca nativa (`src-tauri/`).
`package.json` lista as dependências da interface; `vite.config.ts` configura o build dela;
`index.html` é a página que o programa carrega.

### `public/`
Arquivos servidos como estão (ex.: `icon.png`).

### `src/`: interface

| Pasta / arquivo | O que faz | Exemplos |
|---|---|---|
| `main.tsx` / `App.tsx` | ponto de entrada; rotas e quem pode abrir cada uma | `/login`, `/usuarios` |
| `pages/` | uma tela por arquivo | `SalesPage`, `ProductsPage`, `CustomersPage`, `ReportsPage`, `LoginPage`, `UsersPage`… |
| `components/` | peças usadas em várias telas | `DataGrid` (planilha), `charts` (gráficos), `Tour` (tutorial por papel), `PlansNudge` (lembrete de planos), `PeriodPicker`, `Field`, `Dialog`, `AuthFrame` |
| `layouts/` | molduras: menu lateral do sistema e área de entrada | `AppLayout`, `AuthLayout` |
| `hooks/` | lógica de React compartilhada | `useAuth` (sessão e permissões), `useAsync` (carregar dados) |
| `services/` | chamadas ao núcleo, tipos, textos e formatos | `api.ts` (`call`), `endpoints.ts`, `types.ts`, `labels.ts`, `format.ts` (R$, datas), `period.ts`, `theme.ts` |
| `styles.css` | visual do app inteiro (cores da marca via `--brand*`) | |

### `src-tauri/`: casca nativa

| Item | O que faz |
|---|---|
| `src/main.rs` | inicia o programa (sem janela de console no Windows) |
| `src/lib.rs` | abre o banco, agenda o backup diário e registra os comandos |
| `src/commands/` | um arquivo por módulo; cada comando checa permissão e chama o núcleo |
| `capabilities/` | lista do que a interface pode usar (ex.: diálogo de abrir/salvar arquivo) |
| `icons/` | ícones em todos os tamanhos (Windows `.ico`, macOS `.icns`, PNGs) |
| `tauri.conf.json` | nome, versão, janela, segurança (CSP) e opções do instalador |
| `build.rs` | etapa de build exigida pelo Tauri |
| `gen/` | gerado pelo Tauri (fora do git) |

## `crates/nucleo-core/`: regras e dados

Biblioteca Rust sem interface: dá para testar sem abrir o programa.

| Item | O que faz |
|---|---|
| `migrations/` | SQL que cria e altera as tabelas, numerado (`0001_…`, `0002_…`) e aplicado ao abrir |
| `src/lib.rs` | `Core`: banco aberto + sessão + checagem de permissão (`require`) |
| `src/db.rs` | abre o SQLite, ids (UUID v7), datas, identidade da instalação |
| `src/error.rs` | erros com mensagem pronta para o usuário (`CoreError`) e o `Validator` |
| `src/permissions.rs` | papéis e a matriz do que cada um pode fazer |
| `src/audit.rs` | gravação do histórico (quem mudou o quê) |
| `src/security.rs` | hash e verificação de senha (argon2) |
| `src/validate.rs` | validações comuns: CNPJ, e-mail, cor |
| `src/modules/<m>/` | um módulo de negócio: `models.rs` (dados), `repository.rs` (SQL), `service.rs` (regras) |
| `src/bin/seed.rs` | cria uma empresa de demonstração (`make seed`) |
| `tests/` | testes de integração; `common/` tem os atalhos (`onboarded`, `login_as`) |

Módulos atuais: `auth` (login), `company` (empresa e marca), `users` (pessoas, cadastro,
aprovação), `audit` (histórico), `backup` (cópias), `products` (catálogo, categorias, estoque),
`customers`, `sales` (vendas e cancelamento), `reports` (relatórios) e `spreadsheet`
(exportar .xlsx, importar produtos). `src/money.rs` converte dinheiro e quantidades.

## `docs/`

Documentação por tópico ([índice no README](../README.md#documentação)). `adr/` guarda as
decisões de arquitetura numeradas; uma decisão nova ganha um ADR, e as antigas não são apagadas.

## Gerados (fora do git)

| Pasta | O que é | Pode apagar? |
|---|---|---|
| `target/` | compilação Rust (debug e release, instaladores em `release/bundle/`) | sim (recompila) |
| `.dev-data/` | banco e backups de desenvolvimento (`make seed`, `make dev`) | sim (`make reset-data`) |
| `apps/desktop/node_modules/` | bibliotecas da interface | sim (`make install`) |
| `apps/desktop/dist/` | build da interface | sim |
