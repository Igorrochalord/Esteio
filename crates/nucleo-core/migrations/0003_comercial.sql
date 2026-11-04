-- Comercial: categorias, produtos, estoque (movimentações imutáveis), clientes e vendas.
-- Dinheiro em centavos (INTEGER); quantidades em milésimos (INTEGER, 1 un = 1000; 1,25 kg = 1250).
-- Mesmas regras de sync da 0001: UUID v7, branch/device, timestamps, exclusão lógica e change_log.

CREATE TABLE categories (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    branch_id  TEXT NOT NULL,
    device_id  TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);
CREATE UNIQUE INDEX categories_name_uq ON categories (lower(name)) WHERE deleted_at IS NULL;

CREATE TABLE products (
    id              TEXT PRIMARY KEY,
    code            TEXT NOT NULL,              -- código interno (SKU)
    barcode         TEXT,
    name            TEXT NOT NULL,
    category_id     TEXT REFERENCES categories (id),
    unit            TEXT NOT NULL DEFAULT 'un', -- un, kg, g, l, ml, cx, pct, m
    price_cents     INTEGER NOT NULL DEFAULT 0 CHECK (price_cents >= 0),
    cost_cents      INTEGER NOT NULL DEFAULT 0 CHECK (cost_cents >= 0),
    track_stock     INTEGER NOT NULL DEFAULT 1, -- 0 = não controla estoque (ex.: serviço)
    stock_milli     INTEGER NOT NULL DEFAULT 0, -- saldo em cache; a verdade são as movimentações
    min_stock_milli INTEGER NOT NULL DEFAULT 0,
    active          INTEGER NOT NULL DEFAULT 1,
    branch_id       TEXT NOT NULL,
    device_id       TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    deleted_at      TEXT
);
CREATE UNIQUE INDEX products_code_uq ON products (lower(code)) WHERE deleted_at IS NULL;
CREATE INDEX products_barcode ON products (barcode) WHERE barcode IS NOT NULL;
CREATE INDEX products_category ON products (category_id);

-- Imutável: correções são novas movimentações (ajuste), nunca edição.
CREATE TABLE stock_movements (
    id              TEXT PRIMARY KEY,
    product_id      TEXT NOT NULL REFERENCES products (id),
    kind            TEXT NOT NULL CHECK (kind IN ('entrada', 'saida', 'ajuste', 'venda', 'estorno_venda')),
    qty_milli       INTEGER NOT NULL,           -- variação com sinal (+ entra, - sai)
    balance_milli   INTEGER NOT NULL,           -- saldo depois da movimentação
    unit_cost_cents INTEGER,
    note            TEXT,
    sale_id         TEXT,
    user_id         TEXT,
    branch_id       TEXT NOT NULL,
    device_id       TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    deleted_at      TEXT
);
CREATE INDEX stock_movements_product ON stock_movements (product_id, created_at);
CREATE INDEX stock_movements_sale ON stock_movements (sale_id) WHERE sale_id IS NOT NULL;

CREATE TABLE customers (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    document   TEXT,                            -- CPF ou CNPJ, só dígitos
    phone      TEXT,
    email      TEXT,
    city       TEXT,
    state      TEXT,                            -- UF, ex.: SP
    notes      TEXT,
    active     INTEGER NOT NULL DEFAULT 1,
    branch_id  TEXT NOT NULL,
    device_id  TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);
CREATE UNIQUE INDEX customers_document_uq ON customers (document)
    WHERE document IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX customers_state ON customers (state, city);

CREATE TABLE sales (
    id               TEXT PRIMARY KEY,
    number           INTEGER NOT NULL,          -- sequencial por filial (exibição)
    customer_id      TEXT REFERENCES customers (id),
    seller_id        TEXT NOT NULL REFERENCES users (id),
    status           TEXT NOT NULL DEFAULT 'concluida' CHECK (status IN ('concluida', 'cancelada')),
    payment_method   TEXT NOT NULL CHECK (payment_method IN ('dinheiro', 'pix', 'credito', 'debito', 'fiado', 'outro')),
    subtotal_cents   INTEGER NOT NULL,
    discount_cents   INTEGER NOT NULL DEFAULT 0,
    total_cents      INTEGER NOT NULL,
    cost_total_cents INTEGER NOT NULL,          -- custo dos itens no momento da venda (margem)
    notes            TEXT,
    sold_at          TEXT NOT NULL,
    canceled_at      TEXT,
    canceled_by      TEXT,
    cancel_reason    TEXT,
    branch_id        TEXT NOT NULL,
    device_id        TEXT NOT NULL,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    deleted_at       TEXT
);
CREATE UNIQUE INDEX sales_number_uq ON sales (branch_id, number);
CREATE INDEX sales_sold_at ON sales (sold_at);
CREATE INDEX sales_customer ON sales (customer_id);
CREATE INDEX sales_seller ON sales (seller_id);

CREATE TABLE sale_items (
    id               TEXT PRIMARY KEY,
    sale_id          TEXT NOT NULL REFERENCES sales (id),
    product_id       TEXT NOT NULL REFERENCES products (id),
    product_name     TEXT NOT NULL,             -- foto do nome na hora da venda
    qty_milli        INTEGER NOT NULL CHECK (qty_milli > 0),
    unit_price_cents INTEGER NOT NULL,
    unit_cost_cents  INTEGER NOT NULL,
    discount_cents   INTEGER NOT NULL DEFAULT 0,
    total_cents      INTEGER NOT NULL,
    branch_id        TEXT NOT NULL,
    device_id        TEXT NOT NULL,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    deleted_at       TEXT
);
CREATE INDEX sale_items_sale ON sale_items (sale_id);
CREATE INDEX sale_items_product ON sale_items (product_id);

-- change_log + exclusão lógica (mesmo padrão de users/company)
CREATE TRIGGER categories_cl_insert AFTER INSERT ON categories
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('categories', NEW.id, 'insert'); END;
CREATE TRIGGER categories_cl_update AFTER UPDATE ON categories
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('categories', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER categories_no_delete BEFORE DELETE ON categories
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;

CREATE TRIGGER products_cl_insert AFTER INSERT ON products
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('products', NEW.id, 'insert'); END;
CREATE TRIGGER products_cl_update AFTER UPDATE ON products
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('products', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER products_no_delete BEFORE DELETE ON products
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;

CREATE TRIGGER stock_movements_cl_insert AFTER INSERT ON stock_movements
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('stock_movements', NEW.id, 'insert'); END;
CREATE TRIGGER stock_movements_cl_update AFTER UPDATE ON stock_movements
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('stock_movements', NEW.id, 'update'); END;
CREATE TRIGGER stock_movements_no_delete BEFORE DELETE ON stock_movements
BEGIN SELECT RAISE(ABORT, 'movimentações de estoque são imutáveis'); END;
CREATE TRIGGER stock_movements_no_update BEFORE UPDATE ON stock_movements
BEGIN SELECT RAISE(ABORT, 'movimentações de estoque são imutáveis: lance um ajuste'); END;

CREATE TRIGGER customers_cl_insert AFTER INSERT ON customers
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('customers', NEW.id, 'insert'); END;
CREATE TRIGGER customers_cl_update AFTER UPDATE ON customers
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('customers', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER customers_no_delete BEFORE DELETE ON customers
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;

CREATE TRIGGER sales_cl_insert AFTER INSERT ON sales
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('sales', NEW.id, 'insert'); END;
CREATE TRIGGER sales_cl_update AFTER UPDATE ON sales
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('sales', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER sales_no_delete BEFORE DELETE ON sales
BEGIN SELECT RAISE(ABORT, 'vendas não são apagadas: cancele'); END;

CREATE TRIGGER sale_items_cl_insert AFTER INSERT ON sale_items
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('sale_items', NEW.id, 'insert'); END;
CREATE TRIGGER sale_items_cl_update AFTER UPDATE ON sale_items
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('sale_items', NEW.id, 'update'); END;
CREATE TRIGGER sale_items_no_delete BEFORE DELETE ON sale_items
BEGIN SELECT RAISE(ABORT, 'itens de venda não são apagados: cancele a venda'); END;
