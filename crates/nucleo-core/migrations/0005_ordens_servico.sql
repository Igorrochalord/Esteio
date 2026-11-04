-- Ordens de serviço: OS, itens (serviços e peças = produtos) e linha do tempo.
-- Faturar gera uma venda (baixa de estoque das peças + financeiro). Regras de sync da 0001.

CREATE TABLE service_orders (
    id              TEXT PRIMARY KEY,
    number          INTEGER NOT NULL,          -- sequencial por filial
    customer_id     TEXT NOT NULL REFERENCES customers (id),
    status          TEXT NOT NULL DEFAULT 'aberta'
                    CHECK (status IN ('aberta', 'em_andamento', 'aguardando', 'concluida', 'faturada', 'cancelada')),
    equipment       TEXT,                      -- equipamento/objeto do serviço
    reported        TEXT NOT NULL,             -- problema relatado / pedido do cliente
    diagnosis       TEXT,                      -- diagnóstico e solução
    technician_id   TEXT REFERENCES users (id),
    scheduled_date  TEXT,                      -- AAAA-MM-DD
    due_date        TEXT,                      -- previsão de entrega
    discount_cents  INTEGER NOT NULL DEFAULT 0,
    sale_id         TEXT REFERENCES sales (id),
    finished_at     TEXT,
    canceled_reason TEXT,
    branch_id       TEXT NOT NULL,
    device_id       TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    deleted_at      TEXT
);
CREATE UNIQUE INDEX service_orders_number_uq ON service_orders (branch_id, number);
CREATE INDEX service_orders_status ON service_orders (status);
CREATE INDEX service_orders_customer ON service_orders (customer_id);

CREATE TABLE service_order_items (
    id               TEXT PRIMARY KEY,
    order_id         TEXT NOT NULL REFERENCES service_orders (id),
    product_id       TEXT NOT NULL REFERENCES products (id),
    description      TEXT NOT NULL,
    qty_milli        INTEGER NOT NULL CHECK (qty_milli > 0),
    unit_price_cents INTEGER NOT NULL CHECK (unit_price_cents >= 0),
    discount_cents   INTEGER NOT NULL DEFAULT 0,
    branch_id        TEXT NOT NULL,
    device_id        TEXT NOT NULL,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    deleted_at       TEXT
);
CREATE INDEX service_order_items_order ON service_order_items (order_id);

-- linha do tempo (só inclusão)
CREATE TABLE service_order_events (
    id          TEXT PRIMARY KEY,
    order_id    TEXT NOT NULL REFERENCES service_orders (id),
    user_id     TEXT,
    kind        TEXT NOT NULL CHECK (kind IN ('criada', 'status', 'nota', 'faturada')),
    from_status TEXT,
    to_status   TEXT,
    note        TEXT,
    branch_id   TEXT NOT NULL,
    device_id   TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    deleted_at  TEXT
);
CREATE INDEX service_order_events_order ON service_order_events (order_id, created_at);

CREATE TRIGGER service_orders_cl_insert AFTER INSERT ON service_orders
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('service_orders', NEW.id, 'insert'); END;
CREATE TRIGGER service_orders_cl_update AFTER UPDATE ON service_orders
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('service_orders', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER service_orders_no_delete BEFORE DELETE ON service_orders
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;

CREATE TRIGGER service_order_items_cl_insert AFTER INSERT ON service_order_items
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('service_order_items', NEW.id, 'insert'); END;
CREATE TRIGGER service_order_items_cl_update AFTER UPDATE ON service_order_items
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('service_order_items', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER service_order_items_no_delete BEFORE DELETE ON service_order_items
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;

CREATE TRIGGER service_order_events_cl_insert AFTER INSERT ON service_order_events
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('service_order_events', NEW.id, 'insert'); END;
CREATE TRIGGER service_order_events_cl_update AFTER UPDATE ON service_order_events
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('service_order_events', NEW.id, 'update'); END;
CREATE TRIGGER service_order_events_no_delete BEFORE DELETE ON service_order_events
BEGIN SELECT RAISE(ABORT, 'a linha do tempo da OS não é apagada'); END;
