# Distribuição

## Gerar o instalador

| Onde | Como | Resultado |
|---|---|---|
| CI (recomendado) | push na `main` ou tag `v*` | artifacts `nucleo-erp-windows` e `nucleo-erp-linux` no GitHub Actions |
| Windows local | `make build` (ou `npm run tauri build` em `apps/desktop`) | `target/release/bundle/` |
| Linux / WSL | `make build` | `.AppImage`, `.deb` e `.rpm` em `target/release/bundle/` |

O instalador Windows só é gerado no Windows (ou no runner `windows-latest` do CI), porque
compilar para Windows a partir do Linux não é confiável no Tauri.

Saída no Windows:

```
target/release/
├─ nucleo-erp.exe                              programa sem instalador
└─ bundle/
   ├─ nsis/Nucleo ERP_<versão>_x64-setup.exe   instalador para o cliente
   └─ msi/Nucleo ERP_<versão>_x64_*.msi        instalador para TI / GPO
```

## CI (`.github/workflows/ci.yml`)

1. **lint e testes** (Ubuntu): instala as libs do WebKit, roda `make web-build`, `make lint` e `make test`.
2. **próxima versão** (só na `main`): `scripts/next-version.sh` compara os commits com a última
   tag `vX.Y.Z` (feat → minor, fix → patch; docs/build/test sozinhos → nenhuma).
3. **instalador Windows** (`windows-latest`) e **pacotes Linux** (`ubuntu-22.04`, glibc mais
   antiga para o AppImage rodar em mais distribuições): em PR, só conferem que compila; na `main`,
   rodam quando há versão nova, já com o número gravado (`scripts/set-version.mjs`).
4. **publicar release**: junta os dois, cria a tag `vX.Y.Z` e publica a release (não é rascunho)
   com as notas tiradas dos commits.

Os arquivos saem com **nome fixo**, sem a versão, para o README apontar direto para
`releases/latest/download/<nome>`:

| Arquivo | Origem |
|---|---|
| `NucleoERP-Windows-Instalador.exe` | `bundle/nsis/*-setup.exe` |
| `NucleoERP-Windows.msi` | `bundle/msi/*.msi` |
| `NucleoERP-Linux.AppImage` | `bundle/appimage/*.AppImage` |
| `NucleoERP-Linux.deb` | `bundle/deb/*.deb` |
| `NucleoERP-Linux.rpm` | `bundle/rpm/*.rpm` |

Não renomeie esses arquivos sem atualizar o README e `docs/executar.md`.

## No computador do cliente

| O quê | Onde |
|---|---|
| Programa (NSIS, por usuário) | `%LOCALAPPDATA%\Nucleo ERP\` |
| Programa (MSI) | `C:\Program Files\Nucleo ERP\` |
| Dados | `%APPDATA%\br.com.nucleoerp.desktop\nucleo.db` |
| Cópias de segurança | `%APPDATA%\br.com.nucleoerp.desktop\backups\` |

Programa e dados ficam separados: reinstalar ou atualizar não apaga os dados. O WebView2,
que desenha a interface, já vem no Windows 10 e 11.

## Atalhos

Os instaladores do Tauri já criam os atalhos; não precisa configurar nada:

| Instalador | Menu Iniciar | Área de trabalho |
|---|---|---|
| `…-setup.exe` (NSIS) | sempre | caixa **"Criar atalho na área de trabalho"** na última tela do instalador (marcada pelo usuário); em instalação silenciosa (`/S`) é criado automaticamente |
| `.msi` | sempre | sempre |

O atalho usa o nome `Nucleo ERP` (`productName` em `tauri.conf.json`) e o ícone de
`src-tauri/icons/icon.ico`. Para trocar nome ou ícone, altere esses dois e gere de novo.
Instalação sem atalhos (TI): `…-setup.exe /S /NS`.

## Como as pessoas baixam do GitHub

O repositório é público, então qualquer pessoa baixa os instaladores na página **Releases**:

- Última versão: **https://github.com/Igorrochalord/Esteio/releases/latest**
- Todas as versões: https://github.com/Igorrochalord/Esteio/releases

### Publicar uma versão (você)

Automático: basta enviar commits `feat`/`fix` para a `main` (`make next-version` mostra o número
que vai sair). Para forçar uma versão: **Actions → ci → Run workflow** (`patch`/`minor`/`major`).
Passo a passo e regras em [executar](executar.md#publicar-uma-versão-nova-mantenedor).

Cuidado: tudo que chega na `main` com `feat`/`fix` vira download público em ~20 min. Mudanças
arriscadas devem passar por um PR (o CI compila os instaladores no PR sem publicar).

### Instalar (cliente)

Passo a passo para Windows e Linux em [executar](executar.md#baixar-e-instalar-quem-usa-o-programa).
O aviso *"O Windows protegeu o computador"* some quando houver certificado de assinatura
(ver checklist abaixo).

## Versão

A versão vem de `[workspace.package] version` no `Cargo.toml` da raiz. Mantenha `version`
de `apps/desktop/src-tauri/tauri.conf.json` e `apps/desktop/package.json` iguais a ela.

## Antes da primeira entrega a clientes

- [ ] Certificado de assinatura de código, configurado no CI.
- [ ] Atualização automática (`tauri-plugin-updater`) com chave de assinatura das atualizações.
- [ ] Ícones e nome definitivos.
- [ ] Testar a instalação numa máquina Windows limpa e sem internet.
