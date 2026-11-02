"""fundação: tenants, users, refresh_tokens, audit_logs + RLS

Revision ID: 0001
Revises:
Create Date: 2026-11-02
"""

from collections.abc import Sequence

import sqlalchemy as sa
from sqlalchemy.dialects import postgresql

from alembic import op
from app.core.config import settings
from app.core.rls import APP_ROLE, enable_tenant_rls, grant_app

revision: str = "0001"
down_revision: str | None = None
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None

AUTH_ROLE = settings.db_auth_role


def _now_col(name: str) -> sa.Column:
    return sa.Column(name, sa.DateTime(timezone=True), server_default=sa.func.now(), nullable=False)


def _timestamps() -> list[sa.Column]:
    return [
        _now_col("created_at"),
        _now_col("updated_at"),
    ]


def upgrade() -> None:
    # tenant corrente da transação; NULL se não definido (=> nenhuma linha visível)
    op.execute(
        """
        CREATE FUNCTION app_current_tenant() RETURNS uuid
        LANGUAGE sql STABLE AS
        $$ SELECT NULLIF(current_setting('app.tenant_id', true), '')::uuid $$
        """
    )
    op.execute(f"GRANT EXECUTE ON FUNCTION app_current_tenant() TO {APP_ROLE}")

    op.create_table(
        "tenants",
        sa.Column("id", sa.Uuid(), primary_key=True),
        sa.Column("name", sa.String(200), nullable=False),
        sa.Column("cnpj", sa.String(14)),
        sa.Column("primary_color", sa.String(7), nullable=False),
        sa.Column("secondary_color", sa.String(7), nullable=False),
        sa.Column("logo_key", sa.String(500)),
        sa.Column("plan", sa.String(30), nullable=False),
        sa.Column("subscription_status", sa.String(30), nullable=False),
        *_timestamps(),
    )
    enable_tenant_rls("tenants", column="id")
    grant_app("tenants", "SELECT, INSERT, UPDATE")

    op.create_table(
        "users",
        sa.Column("id", sa.Uuid(), primary_key=True),
        sa.Column("tenant_id", sa.Uuid(), sa.ForeignKey("tenants.id"), nullable=False),
        sa.Column("name", sa.String(200), nullable=False),
        sa.Column("email", sa.String(254), nullable=False),
        sa.Column("password_hash", sa.String(255), nullable=False),
        sa.Column("role", sa.String(20), nullable=False),
        sa.Column("is_active", sa.Boolean(), nullable=False, server_default=sa.true()),
        sa.Column("last_login_at", sa.DateTime(timezone=True)),
        *_timestamps(),
        sa.CheckConstraint("role IN ('owner','admin','user')", name="role_valid"),
    )
    op.create_index("ix_users_tenant_id", "users", ["tenant_id"])
    op.execute("CREATE UNIQUE INDEX uq_users_email_lower ON users (lower(email))")
    # no máximo um owner por empresa
    op.execute("CREATE UNIQUE INDEX uq_users_one_owner ON users (tenant_id) WHERE role = 'owner'")
    enable_tenant_rls("users")
    grant_app("users", "SELECT, INSERT, UPDATE")

    op.create_table(
        "refresh_tokens",
        sa.Column("id", sa.Uuid(), primary_key=True),
        sa.Column("tenant_id", sa.Uuid(), sa.ForeignKey("tenants.id"), nullable=False),
        sa.Column("user_id", sa.Uuid(), sa.ForeignKey("users.id"), nullable=False),
        sa.Column("expires_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("revoked_at", sa.DateTime(timezone=True)),
        _now_col("created_at"),
    )
    op.create_index("ix_refresh_tokens_tenant_id", "refresh_tokens", ["tenant_id"])
    op.create_index("ix_refresh_tokens_user_id", "refresh_tokens", ["user_id"])
    enable_tenant_rls("refresh_tokens")
    grant_app("refresh_tokens", "SELECT, INSERT, UPDATE, DELETE")

    op.create_table(
        "audit_logs",
        sa.Column("id", sa.Uuid(), primary_key=True),
        sa.Column("tenant_id", sa.Uuid(), sa.ForeignKey("tenants.id"), nullable=False),
        sa.Column("user_id", sa.Uuid(), sa.ForeignKey("users.id")),
        sa.Column("action", sa.String(50), nullable=False),
        sa.Column("entity", sa.String(50), nullable=False),
        sa.Column("entity_id", sa.String(64)),
        sa.Column("before", postgresql.JSONB()),
        sa.Column("after", postgresql.JSONB()),
        sa.Column("ip", sa.String(45)),
        _now_col("created_at"),
    )
    op.create_index("ix_audit_logs_tenant_id", "audit_logs", ["tenant_id"])
    op.create_index("ix_audit_logs_created_at", "audit_logs", ["created_at"])
    enable_tenant_rls("audit_logs")
    # append-only: sem UPDATE/DELETE para a aplicação
    grant_app("audit_logs", "SELECT, INSERT")

    # Login: localizar usuário por e-mail ANTES de conhecer o tenant.
    # Único ponto que ignora RLS: função dona = nucleo_auth (BYPASSRLS, NOLOGIN),
    # retorna só o necessário para autenticar.
    op.execute(
        """
        CREATE FUNCTION auth_find_user(p_email text)
        RETURNS TABLE (id uuid, tenant_id uuid, password_hash text, is_active boolean)
        LANGUAGE sql STABLE SECURITY DEFINER
        SET search_path = public, pg_temp
        AS $$
            SELECT u.id, u.tenant_id, u.password_hash::text, u.is_active
            FROM users u WHERE lower(u.email) = lower(p_email)
        $$
        """
    )
    op.execute(f"GRANT SELECT ON users TO {AUTH_ROLE}")
    op.execute(f"ALTER FUNCTION auth_find_user(text) OWNER TO {AUTH_ROLE}")
    op.execute("REVOKE ALL ON FUNCTION auth_find_user(text) FROM PUBLIC")
    op.execute(f"GRANT EXECUTE ON FUNCTION auth_find_user(text) TO {APP_ROLE}")


def downgrade() -> None:
    op.execute("DROP FUNCTION IF EXISTS auth_find_user(text)")
    op.drop_table("audit_logs")
    op.drop_table("refresh_tokens")
    op.drop_table("users")
    op.drop_table("tenants")
    op.execute("DROP FUNCTION IF EXISTS app_current_tenant()")
