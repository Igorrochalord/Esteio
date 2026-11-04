-- Financeiro: contas bancárias, categorias (com grupo da DRE), títulos a pagar/receber e
-- movimentações (extrato). Saldo de uma conta = saldo inicial + soma das movimentações.
-- Mesmas regras de sync da 0001. Dinheiro em centavos; datas de vencimento/movimento AAAA-MM-DD.

CREATE TABLE bank_accounts (
    id                    TEXT PRIMARY KEY,
    name                  TEXT NOT NULL,
    kind                  TEXT NOT NULL CHECK (kind IN ('caixa', 'corrente', 'poupanca', 'carteira')),
    bank_name             TEXT,
    agency                TEXT,
    account_number        TEXT,
    initial_balance_cents INTEGER NOT NULL DEFAULT 0,
    opening_date          TEXT NOT NULL,
    is_default            INTEGER NOT NULL DEFAULT 0,  -- recebe as vendas à vista
    active                INTEGER NOT NULL DEFAULT 1,
    branch_id             TEXT NOT NULL,
    device_id             TEXT NOT NULL,
    created_at            TEXT NOT NULL,
    updated_at            TEXT NOT NULL,
    deleted_at            TEXT
);
CREATE UNIQUE INDEX bank_accounts_name_uq ON bank_accounts (lower(name)) WHERE deleted_at IS NULL;

CREATE TABLE fin_categories (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('receita', 'despesa')),
    -- linha da DRE: receita_vendas, outras_receitas, deducoes, custos, despesas_operacionais,
    -- despesas_pessoal, despesas_financeiras, receitas_financeiras, impostos, investimentos
    dre_group  TEXT NOT NULL,
    system_key TEXT,  -- categorias criadas pelo sistema (ex.: 'vendas'), não editáveis no nome
    active     INTEGER NOT NULL DEFAULT 1,
    branch_id  TEXT NOT NULL,
    device_id  TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);
CREATE UNIQUE INDEX fin_categories_name_uq ON fin_categories (kind, lower(name)) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX fin_categories_system_uq ON fin_categories (system_key) WHERE system_key IS NOT NULL;

-- Títulos: contas a receber e a pagar.
CREATE TABLE fin_entries (
    id                TEXT PRIMARY KEY,
    kind              TEXT NOT NULL CHECK (kind IN ('receber', 'pagar')),
    description       TEXT NOT NULL,
    category_id       TEXT NOT NULL REFERENCES fin_categories (id),
    customer_id       TEXT REFERENCES customers (id),
    counterparty      TEXT,                 -- fornecedor/outro quando não é cliente cadastrado
    amount_cents      INTEGER NOT NULL CHECK (amount_cents > 0),
    issue_date        TEXT NOT NULL,        -- competência (DRE)
    due_date          TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'aberto' CHECK (status IN ('aberto', 'pago', 'cancelado')),
    paid_date         TEXT,
    paid_amount_cents INTEGER,              -- pode diferir (juros, multa, desconto)
    bank_account_id   TEXT REFERENCES bank_accounts (id),
    payment_method    TEXT,
    document          TEXT,                 -- nº do documento/boleto/nota
    installment       INTEGER,              -- parcela n
    installments      INTEGER,              -- de N
    group_id          TEXT,                 -- liga parcelas/recorrências
    sale_id           TEXT REFERENCES sales (id),
    notes             TEXT,
    branch_id         TEXT NOT NULL,
    device_id         TEXT NOT NULL,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    deleted_at        TEXT
);
CREATE INDEX fin_entries_due ON fin_entries (kind, status, due_date);
CREATE INDEX fin_entries_customer ON fin_entries (customer_id);
CREATE INDEX fin_entries_sale ON fin_entries (sale_id) WHERE sale_id IS NOT NULL;

-- Movimentações (extrato). Valor com sinal: + entra, - sai.
CREATE TABLE bank_transactions (
    id              TEXT PRIMARY KEY,
    bank_account_id TEXT NOT NULL REFERENCES bank_accounts (id),
    date            TEXT NOT NULL,
    amount_cents    INTEGER NOT NULL CHECK (amount_cents <> 0),
    description     TEXT NOT NULL,
    category_id     TEXT REFERENCES fin_categories (id),
    entry_id        TEXT REFERENCES fin_entries (id),  -- baixa de título
    sale_id         TEXT REFERENCES sales (id),        -- venda à vista
    transfer_group  TEXT,                               -- liga as duas pontas de uma transferência
    ofx_fitid       TEXT,                               -- id da linha no extrato do banco
    reconciled      INTEGER NOT NULL DEFAULT 0,
    branch_id       TEXT NOT NULL,
    device_id       TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    deleted_at      TEXT
);
CREATE INDEX bank_transactions_account ON bank_transactions (bank_account_id, date);
CREATE UNIQUE INDEX bank_transactions_fitid_uq ON bank_transactions (bank_account_id, ofx_fitid)
    WHERE ofx_fitid IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX bank_transactions_entry ON bank_transactions (entry_id) WHERE entry_id IS NOT NULL;
CREATE INDEX bank_transactions_sale ON bank_transactions (sale_id) WHERE sale_id IS NOT NULL;

-- change_log + exclusão lógica
CREATE TRIGGER bank_accounts_cl_insert AFTER INSERT ON bank_accounts
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('bank_accounts', NEW.id, 'insert'); END;
CREATE TRIGGER bank_accounts_cl_update AFTER UPDATE ON bank_accounts
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('bank_accounts', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER bank_accounts_no_delete BEFORE DELETE ON bank_accounts
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;

CREATE TRIGGER fin_categories_cl_insert AFTER INSERT ON fin_categories
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('fin_categories', NEW.id, 'insert'); END;
CREATE TRIGGER fin_categories_cl_update AFTER UPDATE ON fin_categories
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('fin_categories', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER fin_categories_no_delete BEFORE DELETE ON fin_categories
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;

CREATE TRIGGER fin_entries_cl_insert AFTER INSERT ON fin_entries
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('fin_entries', NEW.id, 'insert'); END;
CREATE TRIGGER fin_entries_cl_update AFTER UPDATE ON fin_entries
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('fin_entries', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER fin_entries_no_delete BEFORE DELETE ON fin_entries
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;

CREATE TRIGGER bank_transactions_cl_insert AFTER INSERT ON bank_transactions
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('bank_transactions', NEW.id, 'insert'); END;
CREATE TRIGGER bank_transactions_cl_update AFTER UPDATE ON bank_transactions
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('bank_transactions', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER bank_transactions_no_delete BEFORE DELETE ON bank_transactions
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;
