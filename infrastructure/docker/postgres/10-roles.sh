#!/bin/sh
# Executado UMA vez, na criação do volume do Postgres (docker-entrypoint-initdb.d).
# Cria:
#   - MIGRATOR_DB_USER: dono dos bancos/schemas, roda DDL (Alembic)
#   - APP_DB_USER:      usado pela API; NOBYPASSRLS, sem DDL
#   - nucleo_auth:      NOLOGIN + BYPASSRLS; só é dono da função auth_find_user()
#                       (login precisa achar o usuário pelo e-mail antes de saber o tenant)
#   - banco <POSTGRES_DB>_test para o pytest
set -eu

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname postgres <<SQL
CREATE ROLE ${MIGRATOR_DB_USER} LOGIN PASSWORD '${MIGRATOR_DB_PASSWORD}' NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
CREATE ROLE ${APP_DB_USER} LOGIN PASSWORD '${APP_DB_PASSWORD}' NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
CREATE ROLE nucleo_auth NOLOGIN BYPASSRLS;
GRANT nucleo_auth TO ${MIGRATOR_DB_USER};

ALTER DATABASE ${POSTGRES_DB} OWNER TO ${MIGRATOR_DB_USER};
CREATE DATABASE ${POSTGRES_DB}_test OWNER ${MIGRATOR_DB_USER};
SQL

for db in "${POSTGRES_DB}" "${POSTGRES_DB}_test"; do
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$db" <<SQL
ALTER SCHEMA public OWNER TO ${MIGRATOR_DB_USER};
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO ${APP_DB_USER};
GRANT USAGE, CREATE ON SCHEMA public TO nucleo_auth;
REVOKE ALL ON DATABASE ${db} FROM PUBLIC;
GRANT CONNECT ON DATABASE ${db} TO ${APP_DB_USER};
SQL
done
