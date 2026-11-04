# Banco de dados

## Onde fica

Um arquivo SQLite, `nucleo.db`, na pasta de dados do app:

| Situação | Pasta |
|---|---|
| Windows (instalado) | `%APPDATA%\br.com.nucleoerp.desktop\` |
| Linux (instalado) | `~/.local/share/br.com.nucleoerp.desktop/` |
| Desenvolvimento | `.dev-data/` (via `NUCLEO_DATA_DIR`) |

Na mesma pasta fica `backups/` ([backup](backup.md)).

Configuração ao abrir (`src/db.rs`): `journal_mode=WAL`, `synchronous=NORMAL`,
`foreign_keys=ON` e `busy_timeout=5s`.

## Migrações

- Arquivos SQL em `crates/nucleo-core/migrations/NNNN_<nome>.sql`, escritos à mão.
- São aplicadas automaticamente ao abrir o app (`sqlx::migrate!`), e ficam embutidas no executável.
- Nunca edite uma migração já distribuída: crie outra.
- Um backup de versão mais nova que o programa é recusado na restauração.

## Regras para toda tabela de negócio

```sql
CREATE TABLE exemplo (
    id         TEXT PRIMARY KEY,      -- UUID v7 (db::new_id), nunca autoincremento
    ...                               -- colunas do negócio
    branch_id  TEXT NOT NULL,         -- filial (installation.branch_id)
    device_id  TEXT NOT NULL,         -- instalação (installation.device_id)
    created_at TEXT NOT NULL,         -- UTC 'YYYY-MM-DDTHH:MM:SS.sssZ' (db::now)
    updated_at TEXT NOT NULL,
    deleted_at TEXT                   -- exclusão lógica
);
-- triggers obrigatórios: <t>_cl_insert, <t>_cl_update (change_log) e <t>_no_delete
```

Copie os triggers de `users` em `0001_fundacao.sql`. O teste `tests/foundation.rs`
confere essas regras em todas as tabelas que não são de infraestrutura.

Outras regras:

- Dinheiro em `INTEGER` (centavos) e quantidades em `INTEGER` milésimos (1 un = 1000,
  1,25 kg = 1250), para `SUM` exato. Na API, `Decimal` serializado como texto (`"12.50"`);
  conversões em `src/money.rs`.
- O saldo de estoque em `products.stock_milli` é cache; a verdade são as `stock_movements`,
  que não podem ser alteradas nem apagadas (correção = nova movimentação de ajuste).
- Consultas filtram `deleted_at IS NULL`.
- Índices únicos levam `WHERE deleted_at IS NULL` (ex.: `users_email_uq`).

## Preparado para sincronizar

As filiais vão trocar dados por um serviço de sincronização numa fase futura. Para não
precisar migrar dados depois, desde já:

- os ids são UUID v7, únicos entre computadores e ordenados pelo tempo;
- cada linha sabe em que filial e em que instalação nasceu;
- `change_log` é o outbox: os triggers registram `insert`/`update`/`delete` de cada linha, e o
  sync vai ler as pendentes (`synced_at IS NULL`) e buscar o estado atual;
- DELETE físico é bloqueado; a exclusão é lógica.

## Tabelas atuais

| Tabela | Tipo | Conteúdo |
|---|---|---|
| `installation` | infra | uma linha: `device_id` e `branch_id` desta instalação |
| `company` | negócio | uma linha: dados, cores e logo (BLOB) |
| `users` | negócio | pessoas, papel, hash argon2, status, `pending` (autocadastro) |
| `categories` | negócio | categorias de produto |
| `products` | negócio | código, preço/custo (centavos), unidade, saldo em cache (milésimos), mínimo |
| `stock_movements` | negócio (imutável) | entrada, saída, ajuste, venda, estorno; saldo após cada uma |
| `customers` | negócio | nome, CPF/CNPJ, contato, cidade, UF (usada no mapa) |
| `sales` | negócio | número por filial, cliente, vendedor, pagamento, totais e custo do momento |
| `sale_items` | negócio | produto (nome congelado), qtd, preço, custo, desconto, total |
| `audit_logs` | infra | histórico append-only |
| `change_log` | infra | outbox do sync |

## Auditoria

- Toda escrita relevante grava em `audit_logs` quem fez, o quê, quando e o antes/depois.
  Só os campos alterados são gravados, e `updated_at` fica de fora.
- Snapshots vêm do `Serialize` do model, e campos com `#[serde(skip_serializing)]`
  (senha, bytes do logo) nunca entram.
- Triggers barram UPDATE e DELETE em `audit_logs`.
- Funções em `src/audit.rs`: `record` (entrada pronta) e `record_changes` (recebe as fotos
  completas e só grava se algo mudou).
