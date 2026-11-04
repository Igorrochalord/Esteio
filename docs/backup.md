# Backup e restauração

Os dados existem só no computador da empresa, então perder o arquivo é o maior risco do
sistema. O código fica em `crates/nucleo-core/src/modules/backup/service.rs`.

## Tipos de cópia

| Tipo | Quando | Arquivo (em `<pasta de dados>/backups/`) |
|---|---|---|
| Automática | ao abrir o app, se a última tiver mais de 24 h; guarda as 7 mais novas | `auto-AAAAMMDD-HHMMSS-mmm.db` |
| Manual | botão "Fazer cópia agora" | `manual-….db` |
| Salvar em… | botão "Salvar cópia em…" (pen drive, pasta da nuvem) | onde o usuário escolher |
| Antes de restaurar | sempre, antes de uma restauração | `pre-restore-….db` |

As cópias usam `VACUUM INTO`: o resultado é um arquivo consistente, gerado mesmo com o app em uso.

## Restauração

Só o proprietário pode restaurar (`backup: manage`), de uma cópia da lista ou de um arquivo.

1. O arquivo é copiado para uma área temporária e conferido: precisa ser SQLite íntegro
   (`integrity_check`), ter as tabelas do Núcleo e não ser de versão mais nova que o programa.
2. O estado atual é salvo em `pre-restore-….db`.
3. O banco é fechado e substituído, e é reaberto com as migrações aplicadas.
4. Se a reabertura falhar, o estado anterior volta automaticamente.
5. A sessão é encerrada e o histórico registra `backup.restore`.

## Integrações

A cópia leva a configuração das integrações, mas **não** as credenciais utilizáveis: elas
ficam cifradas com a chave `integracoes.key`, que fica na pasta de dados e não entra na
cópia. Restaurada no mesmo computador, nada muda. Em outro computador, a tela de
Integrações pede os dados de acesso de novo.

## Recomendação ao cliente

Guardar uma cópia **fora do computador** com frequência (botão "Salvar cópia em…"). As
cópias automáticas ficam no mesmo disco e não protegem contra defeito ou roubo da máquina.
