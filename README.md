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

## Como atualizar

### Para quem usa o programa (clientes)

1. **Faça uma cópia de segurança antes** (menu *Cópias de segurança* → *Salvar cópia em…*).
2. Baixe a versão nova em **[Releases](https://github.com/Igorrochalord/Esteio/releases/latest)**
   (arquivo `Nucleo ERP_<versão>_x64-setup.exe`).
3. Feche o Núcleo ERP e execute o instalador **por cima** da versão atual. Não precisa desinstalar.
4. Abra o programa. Seus dados continuam lá: ficam em
   `%APPDATA%\br.com.nucleoerp.desktop\` (separados do programa), e o banco é atualizado
   sozinho na primeira abertura (migrações automáticas).

Não volte para uma versão mais antiga depois de abrir a nova: o banco já foi atualizado e
o programa antigo recusa backups de versão mais nova. Em caso de problema, instale de novo a
versão nova e, se preciso, restaure a cópia feita no passo 1.

### Para você: publicar uma atualização

```bash
make check                       # tudo verde antes de publicar
make version v=0.2.0             # sobe a versão nos 3 arquivos
git commit -am "build: versão 0.2.0"
git tag v0.2.0
git push origin main --tags      # o CI gera o instalador e um release em rascunho
```

No GitHub, abra **Releases**, revise o rascunho, escreva as novidades e clique em
**Publish release**. O link "latest" passa a apontar para a versão nova.
Atualização automática dentro do programa (sem baixar à mão) está prevista: `tauri-plugin-updater`
com chave de assinatura (ver [distribuição](docs/distribuicao.md)).

### Para quem desenvolve: atualizar o código local

```bash
git pull
make install     # se o package.json mudou
make check       # fmt, lint, tipos e testes
make dev         # migrações novas são aplicadas sozinhas ao abrir
```

Se a mudança trouxe dados de demonstração novos: `make reset-data && make seed`.

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
