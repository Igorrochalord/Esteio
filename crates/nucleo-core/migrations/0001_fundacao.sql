-- Fundação local (ADR 0002). Regras para toda tabela de negócio:
--   id TEXT (UUID v7), branch_id, device_id, created_at, updated_at, deleted_at (exclusão lógica),
--   triggers de change_log (outbox para o sync futuro) e proibição de DELETE físico.
-- Datas: TEXT UTC 'YYYY-MM-DDTHH:MM:SS.sssZ' (ordenável como texto).

-- Esta instalação (uma linha). Gerada pelo app na primeira abertura.
CREATE TABLE installation (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    device_id  TEXT NOT NULL,
    branch_id  TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Outbox de alterações: o sync lê daqui e busca o estado atual da linha.
CREATE TABLE change_log (
    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
    entity     TEXT NOT NULL,
    entity_id  TEXT NOT NULL,
    op         TEXT NOT NULL CHECK (op IN ('insert', 'update', 'delete')),
    changed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    synced_at  TEXT
);
CREATE INDEX change_log_pending ON change_log (seq) WHERE synced_at IS NULL;

-- Empresa: uma única linha por instalação.
CREATE TABLE company (
    id              TEXT PRIMARY KEY,
    singleton       INTEGER NOT NULL DEFAULT 1 UNIQUE CHECK (singleton = 1),
    name            TEXT NOT NULL,
    cnpj            TEXT,
    primary_color   TEXT NOT NULL,
    secondary_color TEXT NOT NULL,
    logo            BLOB,
    logo_type       TEXT,
    logo_id         TEXT,
    branch_id       TEXT NOT NULL,
    device_id       TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    deleted_at      TEXT
);

CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    email         TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'user')),
    is_active     INTEGER NOT NULL DEFAULT 1,
    last_login_at TEXT,
    branch_id     TEXT NOT NULL,
    device_id     TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    deleted_at    TEXT
);
CREATE UNIQUE INDEX users_email_uq ON users (lower(email)) WHERE deleted_at IS NULL;

-- Auditoria append-only.
CREATE TABLE audit_logs (
    id         TEXT PRIMARY KEY,
    user_id    TEXT,
    action     TEXT NOT NULL,
    entity     TEXT NOT NULL,
    entity_id  TEXT,
    before     TEXT,
    after      TEXT,
    branch_id  TEXT NOT NULL,
    device_id  TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX audit_logs_recent ON audit_logs (created_at DESC, id DESC);
CREATE INDEX audit_logs_entity ON audit_logs (entity, entity_id);

CREATE TRIGGER audit_logs_no_update BEFORE UPDATE ON audit_logs
BEGIN SELECT RAISE(ABORT, 'audit_logs é somente leitura'); END;
CREATE TRIGGER audit_logs_no_delete BEFORE DELETE ON audit_logs
BEGIN SELECT RAISE(ABORT, 'audit_logs é somente leitura'); END;

-- change_log + exclusão lógica: company
CREATE TRIGGER company_cl_insert AFTER INSERT ON company
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('company', NEW.id, 'insert'); END;
CREATE TRIGGER company_cl_update AFTER UPDATE ON company
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('company', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER company_no_delete BEFORE DELETE ON company
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;

-- change_log + exclusão lógica: users
CREATE TRIGGER users_cl_insert AFTER INSERT ON users
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('users', NEW.id, 'insert'); END;
CREATE TRIGGER users_cl_update AFTER UPDATE ON users
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('users', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER users_no_delete BEFORE DELETE ON users
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;
