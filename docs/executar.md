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

```bash
make check                       # tudo verde antes de publicar
make version v=0.2.0             # sobe a versão nos 3 arquivos
git commit -am "build: versão 0.2.0"
git tag v0.2.0
git push origin main --tags      # o CI gera Windows + Linux e um release em rascunho
```

No GitHub, abra **Releases**, revise o rascunho, escreva as novidades e clique em
**Publish release**. Os links "latest" do README passam a baixar a versão nova, porque o
CI publica os arquivos sempre com o mesmo nome (`NucleoERP-Windows-Instalador.exe`,
`NucleoERP-Linux.AppImage`, …).

Atualização automática dentro do programa (sem baixar à mão) está prevista:
`tauri-plugin-updater` com chave de assinatura (ver [distribuição](distribuicao.md)).
