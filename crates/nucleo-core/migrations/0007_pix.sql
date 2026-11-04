-- PIX na venda (fase 6b): cada cobrança liga a venda, o título a receber e o provedor.
-- O QR é gerado localmente a partir do copia-e-cola; a baixa acontece quando a consulta
-- ao provedor confirma o pagamento (sem webhook, ADR 0003). Regras de sync da 0001.

CREATE TABLE pix_charges (
    id             TEXT PRIMARY KEY,
    sale_id        TEXT NOT NULL REFERENCES sales (id),
    entry_id       TEXT NOT NULL REFERENCES fin_entries (id),
    integration_id TEXT NOT NULL REFERENCES integrations (id),
    provider       TEXT NOT NULL,                 -- id do provedor (ex.: 'mercadopago')
    amount_cents   INTEGER NOT NULL CHECK (amount_cents > 0),
    status         TEXT NOT NULL DEFAULT 'gerando'
                   CHECK (status IN ('gerando', 'aguardando', 'pago', 'expirado', 'cancelado', 'erro')),
    txid           TEXT NOT NULL,                 -- nosso identificador (26–35 letras/números)
    provider_ref   TEXT,                          -- id da cobrança no provedor
    copy_paste     TEXT,                          -- PIX copia-e-cola (BR Code)
    expires_at     TEXT,
    paid_at        TEXT,
    error          TEXT,                          -- última falha, em texto para o usuário
    branch_id      TEXT NOT NULL,
    device_id      TEXT NOT NULL,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    deleted_at     TEXT
);
CREATE INDEX pix_charges_sale ON pix_charges (sale_id);
CREATE INDEX pix_charges_status ON pix_charges (status);
CREATE UNIQUE INDEX pix_charges_txid_uq ON pix_charges (txid);

CREATE TRIGGER pix_charges_cl_insert AFTER INSERT ON pix_charges
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('pix_charges', NEW.id, 'insert'); END;
CREATE TRIGGER pix_charges_cl_update AFTER UPDATE ON pix_charges
BEGIN INSERT INTO change_log (entity, entity_id, op) VALUES ('pix_charges', NEW.id,
    CASE WHEN NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN 'delete' ELSE 'update' END); END;
CREATE TRIGGER pix_charges_no_delete BEFORE DELETE ON pix_charges
BEGIN SELECT RAISE(ABORT, 'use exclusão lógica (deleted_at)'); END;
