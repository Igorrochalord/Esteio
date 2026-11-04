# Executar, instalar e atualizar

Tudo o que é "como rodar" fica aqui. O [README](../README.md) só apresenta o projeto e os
links de download.

## Baixar e instalar (quem usa o programa)

Todos os arquivos ficam em **[Releases](https://github.com/Igorrochalord/Esteio/releases/latest)**.
Baixe **um** arquivo, conforme o seu computador:

| Seu computador | Arquivo | Link direto |
|---|---|---|
| Windows 10 ou 11 | `NucleoERP-Windows-Instalador.exe` | [baixar](https://github.com/Igorrochalord/Esteio/releases/latest/download/NucleoERP-Windows-Instalador.exe) |
| Linux (qualquer distribuição) | `NucleoERP-Linux.AppImage` | [baixar](https://github.com/Igorrochalord/Esteio/releases/latest/download/NucleoERP-Linux.AppImage) |
| Ubuntu, Debian, Mint, Pop!_OS | `NucleoERP-Linux.deb` | [baixar](https://github.com/Igorrochalord/Esteio/releases/latest/download/NucleoERP-Linux.deb) |
| Fedora, openSUSE | `NucleoERP-Linux.rpm` | [baixar](https://github.com/Igorrochalord/Esteio/releases/latest/download/NucleoERP-Linux.rpm) |
| Windows em empresa (TI, GPO) | `NucleoERP-Windows.msi` | [baixar](https://github.com/Igorrochalord/Esteio/releases/latest/download/NucleoERP-Windows.msi) |

Na dúvida no Linux, use o **AppImage**: não instala nada, é só dar permissão e abrir.

### Windows

1. Baixe `NucleoERP-Windows-Instalador.exe` e execute.
2. Enquanto o instalador não for assinado digitalmente, o Windows mostra
   *"O Windows protegeu o computador"*: clique em **Mais informações → Executar assim mesmo**.
3. Siga o instalador (dá para marcar o atalho na área de trabalho) e abra o **Nucleo ERP**.
4. Na primeira abertura, o sistema pede o cadastro da empresa e do proprietário.

### Linux: AppImage

```bash
chmod +x NucleoERP-Linux.AppImage
./NucleoERP-Linux.AppImage
```

Ou, no gerenciador de arquivos: botão direito → *Propriedades* → *Permitir executar como
programa* → dois cliques. Se não abrir em distribuições mais novas, instale o FUSE
(`sudo apt install libfuse2` no Ubuntu).

### Linux: .deb / .rpm

```bash
sudo apt install ./NucleoERP-Linux.deb      # Ubuntu, Debian, Mint
sudo dnf install ./NucleoERP-Linux.rpm      # Fedora
```

Depois é só procurar **Nucleo ERP** no menu de aplicativos. Dois cliques no `.deb` também
funcionam na maioria das distribuições (abre a central de programas).

### Onde ficam os dados

| Sistema | Pasta |
|---|---|
| Windows | `%APPDATA%\br.com.nucleoerp.desktop\` |
| Linux | `~/.local/share/br.com.nucleoerp.desktop/` |

Programa e dados ficam separados: reinstalar ou atualizar não apaga os dados.

## Atualizar (quem usa o programa)

1. **Faça uma cópia de segurança antes** (menu *Cópias de segurança* → *Salvar cópia em…*).
2. Baixe a versão nova pelos links acima (eles sempre apontam para a última versão).
3. Feche o Núcleo ERP e instale **por cima** da versão atual. Não precisa desinstalar.
   No AppImage, basta trocar o arquivo antigo pelo novo.
4. Abra o programa. Os dados continuam lá, e o banco é atualizado sozinho na primeira
   abertura (migrações automáticas).

Não volte para uma versão mais antiga depois de abrir a nova: o banco já foi atualizado e
o programa antigo recusa backups de versão mais nova. Em caso de problema, instale de novo a
versão nova e, se preciso, restaure a cópia feita no passo 1.

## Rodar a partir do código (quem desenvolve)

Pré-requisitos (Rust, Node, libs do WebKit): [desenvolvimento](desenvolvimento.md).

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

### Atualizar o código local

```bash
git pull
make install     # se o package.json mudou
make check       # fmt, lint, tipos e testes
make dev         # migrações novas são aplicadas sozinhas ao abrir
```

Se a mudança trouxe dados de demonstração novos: `make reset-data && make seed`.

### Gerar o instalador na própria máquina

`make build` gera o instalador da plataforma atual em `target/release/bundle/`
(Windows: `.exe` e `.msi`; Linux/WSL: `.AppImage`, `.deb` e `.rpm`). Detalhes em
[distribuição](distribuicao.md).

## Publicar uma versão nova (mantenedor)

**É automático.** A cada push na `main`, o CI (`.github/workflows/ci.yml`) olha os commits
desde a última versão (`vX.Y.Z`) e decide pela mensagem (Conventional Commits):

| Commits desde a última versão | Versão nova |
|---|---|
| algum `feat(...)` | sobe o meio: 0.3.0 → 0.4.0 |
| só `fix(...)`/`perf(...)` | sobe o final: 0.3.0 → 0.3.1 |
| `feat!:`/`BREAKING CHANGE` | sobe o primeiro (enquanto for 0.x, sobe o meio) |
| só `docs`, `build`, `test`, `chore`… | nenhuma (não gera instalador) |

Com versão nova, o CI grava o número nos 3 arquivos (`scripts/set-version.mjs`), gera Windows e
Linux, cria a tag `vX.Y.Z` e **publica** a release com as notas (novidades e correções tiradas
dos commits). Os links "latest" do README passam a baixar a versão nova, porque os arquivos têm
sempre o mesmo nome (`NucleoERP-Windows-Instalador.exe`, `NucleoERP-Linux.AppImage`, …).

```bash
make check          # tudo verde antes de enviar
make next-version   # mostra qual versão o próximo push vai publicar
git push origin main
```

Para forçar uma versão (ex.: só correções de texto que você quer distribuir): GitHub →
**Actions → ci → Run workflow** e escolha `patch`, `minor` ou `major`. Não crie tags à mão: a
release cria a tag. Os arquivos do repositório podem continuar com a versão antiga; quem manda é
a última tag `vX.Y.Z`.

Atualização automática dentro do programa (sem baixar à mão) está prevista:
`tauri-plugin-updater` com chave de assinatura (ver [distribuição](distribuicao.md)).
