# Segurança e permissões

## Login e sessão

- O primeiro usuário é o **proprietário**, criado no onboarding junto com a empresa. Os demais são
  cadastrados por quem tem `users: manage`.
- **Autocadastro:** em "Pedir cadastro", a pessoa cria a própria conta, que nasce **inativa e
  aguardando aprovação** (`pending`). Quem tem `users: manage` aprova (escolhendo o papel) ou
  recusa em Pessoas. A recusa é uma exclusão lógica e libera o e-mail para um novo pedido. O aviso
  "aguardando aprovação" só aparece para quem acertou a senha.
- **Esqueci a senha:** um administrador define uma senha provisória em Pessoas → "Redefinir senha".
  Ninguém redefine a própria senha por ali, e só o proprietário redefine a do proprietário.
  O histórico registra o fato, nunca a senha.
- O e-mail é único (sem diferenciar maiúsculas) e é gravado em minúsculas.
- As senhas usam **argon2**. A verificação roda fora do executor async e leva o mesmo tempo
  quando o e-mail não existe, para não revelar quais e-mails estão cadastrados.
  A mensagem de erro é sempre a mesma: "E-mail ou senha inválidos".
- A sessão é o id do usuário **na memória do processo**. Não há token salvo em disco, e
  fechar o app encerra a sessão.
- Cada comando relê o usuário no banco (`Core::current`), então desativar alguém corta o acesso na hora.
- Restaurar um backup encerra a sessão, porque os usuários podem ser outros.

## Papéis

| Módulo | Proprietário | Administrador | Usuário |
|---|---|---|---|
| settings (empresa, cores, logo) | manage | manage | read |
| users (pessoas) | manage | manage | — |
| audit (histórico) | manage | read | — |
| backup | manage (restaura) | write (faz cópia) | — |
| products (catálogo e estoque) | manage | manage | read (sem custo) |
| customers | manage | manage | write |
| sales | manage | manage (cancela) | write (vende) |
| reports (inclui custo, margem, vendedores) | manage | read | — |

Detalhes do comercial: quem não pode editar produtos não recebe o custo; lucro/custo da venda
só para quem vê relatórios; alterar preço unitário na venda e lançar venda com data passada
exigem `sales: manage`; cancelar venda exige `sales: manage` e devolve o estoque.

Os níveis se acumulam: `manage` inclui `write`, que inclui `read`. A matriz fica em
`crates/nucleo-core/src/permissions.rs` (`granted`). O front recebe a matriz efetiva em
`me.permissions` para montar o menu, mas **quem decide é o núcleo**.

Regras de pessoas (`users::service`):

- o proprietário não pode ser rebaixado nem desativado;
- ninguém altera o próprio papel ou status;
- só o proprietário edita os próprios dados;
- o papel `owner` não pode ser atribuído depois do onboarding;
- cadastro pendente só muda por Aprovar/Recusar (não dá para "reativar" e pular a aprovação).

## Superfície do app

- **CSP** (`tauri.conf.json`): `connect-src` só para o IPC do Tauri, então a interface não
  consegue acessar a rede. Imagens só locais, `data:` ou `blob:`.
- **Capabilities** (`src-tauri/capabilities/default.json`): `core:default` mais os diálogos
  de abrir, salvar e confirmar.
- **Logo**: só PNG e JPEG, conferidos pela assinatura dos bytes, até 2 MB. SVG é recusado
  porque pode conter script.

## Pendências antes de distribuir

- Assinar o instalador (certificado de code signing), para evitar o alerta do SmartScreen.
- O banco não é criptografado. Se for exigido, avaliar SQLCipher.
