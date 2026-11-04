# Núcleo ERP

ERP modular para pequenas e médias empresas, distribuído como **programa instalável**
(`.exe` no Windows). Cada empresa instala no próprio computador e os dados ficam num arquivo
local. Não há servidor nem internet envolvidos, e o sistema assume as cores e o logo da empresa.

| Parte | Tecnologia |
|---|---|
| Interface | React 19 + TypeScript + Vite, dentro do Tauri v2 |
| Regras de negócio | Rust (`crates/nucleo-core`) |
| Dados | SQLite local (sqlx), com backup automático |
| Instalador | Tauri bundler: NSIS/MSI no Windows, gerado no CI |

## Baixar

Instalador para Windows na página de versões:
**[github.com/Igorrochalord/Esteio/releases/latest](https://github.com/Igorrochalord/Esteio/releases/latest)**
(baixe o `…-setup.exe`). Passo a passo e atalhos em [distribuição](docs/distribuicao.md).

## Começando

Pré-requisitos e instalação do ambiente: [docs/desenvolvimento.md](docs/desenvolvimento.md).

```bash
make install   # dependências do frontend
make test      # testes do núcleo
make seed      # empresa de demonstração em .dev-data/
make dev       # abre o app usando .dev-data/
```

Usuários do seed (senha `nucleo123`): `dono@paodourado.com.br` (proprietário),
`admin@paodourado.com.br` (administrador), `caixa@paodourado.com.br` e `balcao@paodourado.com.br`
(usuários). O seed também cria 28 produtos, 24 clientes e 90 dias de vendas.
Para a outra empresa de exemplo: `make reset-data && make seed empresa=techparts`.

## Estrutura

Resumo abaixo; explicação de cada pasta em [docs/estrutura.md](docs/estrutura.md).

```
apps/desktop/            app instalável
  src/                   interface React (pages, components, layouts, services, hooks)
  src-tauri/             casca Tauri: comandos finos que chamam o núcleo
crates/nucleo-core/      regras de negócio + banco (sem interface)
  src/modules/<m>/       models, repository, service de cada módulo
  migrations/            migrações SQL do SQLite
  tests/                 testes de integração
docs/                    documentação por tópico + decisões (adr/)
.github/workflows/       CI: lint, testes e instalador Windows
```

## Documentação

**Documentação Master em PDF:** [docs/Nucleo-ERP-Documentacao.pdf](docs/Nucleo-ERP-Documentacao.pdf), com o projeto inteiro,
arquivo por arquivo, e onde alterar cada coisa (fonte: `docs/manual/manual.html`; regenerar com `make manual`).

| Tópico | Conteúdo |
|---|---|
| [Estrutura de pastas](docs/estrutura.md) | o que cada pasta e arquivo principal faz |
| [Arquitetura](docs/arquitetura.md) | camadas, caminho de uma ação da tela ao banco, módulos |
| [Desenvolvimento](docs/desenvolvimento.md) | ambiente, comandos, testes, como criar um módulo |
| [Banco de dados](docs/banco-de-dados.md) | SQLite, migrações, regras de tabela, sync, auditoria |
| [Segurança e permissões](docs/seguranca-e-permissoes.md) | login, sessão, papéis, CSP |
| [Backup e restauração](docs/backup.md) | cópias automáticas e manuais, restauração |
| [Distribuição](docs/distribuicao.md) | instalador, atalhos, publicar e baixar pelo GitHub |
| [Assinatura](docs/assinatura.md) | pesquisa: modelos de plano, cobrança e licença offline |
| [Roteiro](docs/roteiro.md) | fases concluídas e próximas |
| [Decisões (ADR)](docs/adr/) | registro das decisões de arquitetura |
