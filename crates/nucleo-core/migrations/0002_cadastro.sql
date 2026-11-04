-- Autocadastro: a pessoa pede acesso pela tela de login e fica aguardando aprovação
-- (is_active = 0, pending = 1) até um administrador aprovar ou recusar.
ALTER TABLE users ADD COLUMN pending INTEGER NOT NULL DEFAULT 0;
