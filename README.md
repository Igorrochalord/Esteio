# nucleo-erp

ERP modular vendido como SaaS. Cada empresa (tenant) tem seus dados isolados por
Row-Level Security no PostgreSQL e o sistema assume as cores e o logo dela.

| Parte | Stack |
|---|---|
| API | Python 3.12, FastAPI, SQLAlchemy 2 async, Alembic, Pydantic v2, uv |
| Banco | PostgreSQL 16, banco compartilhado + RLS por `tenant_id` |
| Arquivos | S3 (SeaweedFS em dev, AWS S3 em produção) via boto3 |
| Desktop | Tauri v2 + React + TypeScript + Vite + React Router (só interface) |

## Começando

Pré-requisitos: Docker com Compose v2 e `make`. Node e Python locais são opcionais:
lint, testes e o frontend rodam em containers.

```bash
make up        # cria .env a partir do .env.example, sobe db + storage + api e espera ficar saudável
make seed      # duas empresas de exemplo
make smoke     # teste ponta a ponta via HTTP
make web-dev   # frontend no navegador em http://localhost:1420
```

- API: http://localhost:8000 · documentação interativa: http://localhost:8000/docs
- Saúde: http://localhost:8000/health
- S3 (SeaweedFS): http://localhost:8333

### Usuários do seed (senha `nucleo123`)

| Empresa | E-mail | Papel |
|---|---|---|
| Padaria Pão Dourado | dono@paodourado.com.br | owner |
| | admin@paodourado.com.br | admin |
| | caixa@paodourado.com.br | user |
| TechParts Distribuidora | dono@techparts.com.br | owner |
| | vendas@techparts.com.br | user |

## Comandos

| Comando | O que faz |
|---|---|
| `make up` / `make down` | sobe / para o ambiente (volumes preservados) |
| `make reset` | **apaga os volumes** e recria tudo do zero |
| `make logs s=api` | segue os logs de um serviço |
| `make test` | pytest dentro do container (banco `nucleo_test`, conectando como usuário sem bypass de RLS) |
| `make lint` | ruff + mypy (backend), tsc + eslint (desktop) |
| `make fmt` | formata o backend |
| `make smoke` | `scripts/smoke.sh` contra a API rodando |
| `make seed` | `scripts/seed.py` (idempotente) |
| `make psql` / `make shell` | psql como superusuário / shell no container da api |
| `make web-dev` / `make web-build` | frontend em dev / build de produção |

## Desktop (Tauri)

O frontend roda no navegador para desenvolvimento. Para o app nativo:

```bash
cd apps/desktop
npm install
npm run tauri dev     # requer Rust + dependências do Tauri no SO
npm run tauri build   # instalador do SO atual
```

O instalador Windows deve ser gerado no Windows (ou no CI, fase 7). A URL da API vem de
`VITE_API_URL` e pode ser trocada na tela de login (link "Alterar" ao lado de "Servidor").

## Estrutura

```
apps/backend/      app/core (config, db, segurança, RBAC, storage, auditoria)
                   app/modules/<módulo>/{models,schemas,repository,service,router}.py
                   alembic/ (migrações), tests/
apps/desktop/      src/{pages,components,layouts,services,hooks}, src-tauri/
infrastructure/    docker/ (compose + init do Postgres), terraform/
scripts/           smoke.sh, seed.py
docs/adr/          decisões de arquitetura
```

Veja [docs/adr/0001-arquitetura.md](docs/adr/0001-arquitetura.md) para as decisões.
