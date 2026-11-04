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

## Versão

A versão vem de `[workspace.package] version` no `Cargo.toml` da raiz. Mantenha `version`
de `apps/desktop/src-tauri/tauri.conf.json` e `apps/desktop/package.json` iguais a ela.

## Antes da primeira entrega a clientes

- [ ] Certificado de assinatura de código, configurado no CI.
- [ ] Atualização automática (`tauri-plugin-updater`) com chave de assinatura das atualizações.
- [ ] Ícones e nome definitivos.
- [ ] Testar a instalação numa máquina Windows limpa e sem internet.
