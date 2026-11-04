-- Integrações online (ADR 0003): conexões com APIs externas e fila de trabalhos.
-- O app funciona sem internet; o que depende de API entra na fila e roda quando houver conexão.
-- Regras de sync da 0001.

CREATE TABLE integrations (
    id            TEXT PRIMARY KEY,
    provider      TEXT NOT NULL,                 -- id do catálogo (ex.: 'mercadopago')
    environment   TEXT NOT NULL DEFAULT 'sandbox'
                  CHECK (environment IN ('sandbox', 'production')),
    enabled       INTEGER NOT NULL DEFAULT 0,
    config        TEXT NOT NULL DEFAULT '{}',    -- JSON sem segredos
    secrets       BLOB,                          -- JSON cifrado (chave fora do banco)
    status        TEXT NOT NULL DEFAULT 'unchecked'
                  CHECK (status IN ('unchecked', 'ok', 'error')),
    status_detail TEXT,
    checked_at    TEXT,
    branch_id     TEXT NOT NULL,
    device_id     TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    deleted_at    TEXT
);
CREATE UNIQUE INDEX integrations_provider_uq ON integrations (provider) WHERE deleted_at IS NULL;

-- fila de trabalhos (outbox de saída para as APIs)
CREATE TABLE integration_jobs (
    id              TEXT PRIMARY KEY,
    integration_id  TEXT NOT NULL REFERENCES integrations (id),
    kind            TEXT NOT NULL,               -- ex.: 'pix.create', 'pix.poll'
    payload         TEXT NOT NULL DEFAULT '{}',  -- JSON
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'running', 'done', 'failed', 'canceled')),
    attempts        INTEGER NOT NULL DEFAULT 0,  -- falhas seguidas
    next_run_at     TEXT NOT NULL,
    last_error      TEXT,
    result          TEXT,                        -- JSON
    ref_entity      TEXT,                        -- ex.: 'sales'
    ref_id          TEXT,
    finished_at     TEXT,
    branch_id       TEXT NOT NULL,
    device_id       TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    deleted_at      TEXT
);
CREATE INDEX integration_jobs_due ON integration_jobs (status, next_run_at);
CREATE INDEX integration_jobs_ref ON integration_jobs (ref_entity, ref_id);

CREATE TRIGGER integrations_cl_insert AFTER INSERT ON integrations
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('integrations', NEW.id, 'insert'); END;
CREATE TRIGGER integrations_cl_update AFTER UPDATE ON integrations
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('integrations', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER integrations_no_delete BEFORE DELETE ON integrations
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;

CREATE TRIGGER integration_jobs_cl_insert AFTER INSERT ON integration_jobs
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('integration_jobs', NEW.id, 'insert'); END;
CREATE TRIGGER integration_jobs_cl_update AFTER UPDATE ON integration_jobs
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('integration_jobs', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER integration_jobs_no_delete BEFORE DELETE ON integration_jobs
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;
