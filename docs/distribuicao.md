# Distribuição

## Gerar o instalador

| Onde | Como | Resultado |
|---|---|---|
| CI (recomendado) | push na `main` ou tag `v*` | artifact `nucleo-erp-windows` no GitHub Actions |
| Windows local | `make build` (ou `npm run tauri build` em `apps/desktop`) | `target/release/bundle/` |
| Linux / WSL | `make build` | `.deb` e `.AppImage` em `target/release/bundle/` |

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
2. **instalador Windows**: só roda se o passo 1 passar; gera NSIS e MSI e publica como artifact.
3. **release**: numa tag `v*` (ex.: `git tag v0.2.0 && git push --tags`), cria um release em
   rascunho no GitHub com os instaladores anexados.

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

1. Suba a versão em `Cargo.toml` (`[workspace.package] version`), `tauri.conf.json` e
   `package.json` (ex.: `0.2.0`) e faça o commit.
2. Crie e envie a tag:
   ```bash
   git tag v0.2.0
   git push origin main --tags
   ```
3. O CI testa, gera o instalador no Windows e cria um **release em rascunho** com o
   `…-setup.exe` e o `.msi` anexados.
4. No GitHub, abra **Releases**, revise o rascunho, escreva as novidades e clique em
   **Publish release**. A partir daí o link "latest" aponta para ela.

O rascunho existe para você conferir antes que o público veja. Uma tag errada não vira
download automaticamente.

### Instalar (cliente)

1. Abra o link da última versão e baixe **`Nucleo ERP_<versão>_x64-setup.exe`**.
2. Execute. Enquanto o instalador não for assinado digitalmente, o Windows mostra
   *"O Windows protegeu o computador"*: clique em **Mais informações → Executar assim mesmo**.
   Isso some quando houver certificado de assinatura (ver checklist abaixo).
3. Siga o instalador (dá para marcar o atalho na área de trabalho) e abra o **Nucleo ERP**.
4. Na primeira abertura, o sistema pede o cadastro da empresa e do proprietário.

## Versão

A versão vem de `[workspace.package] version` no `Cargo.toml` da raiz. Mantenha `version`
de `apps/desktop/src-tauri/tauri.conf.json` e `apps/desktop/package.json` iguais a ela.

## Antes da primeira entrega a clientes

- [ ] Certificado de assinatura de código, configurado no CI.
- [ ] Atualização automática (`tauri-plugin-updater`) com chave de assinatura das atualizações.
- [ ] Ícones e nome definitivos.
- [ ] Testar a instalação numa máquina Windows limpa e sem internet.
