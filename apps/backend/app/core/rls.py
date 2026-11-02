"""Helpers de migração (Alembic) para Row-Level Security. Use em toda tabela de negócio."""

from alembic import op
from app.core.config import settings

APP_ROLE = settings.db_app_role


def enable_tenant_rls(table: str, column: str = "tenant_id") -> None:
    """Liga e FORÇA RLS: só linhas onde <column> = tenant da transação."""
    op.execute(f"ALTER TABLE {table} ENABLE ROW LEVEL SECURITY")
    # FORCE: nem o dono da tabela escapa da policy
    op.execute(f"ALTER TABLE {table} FORCE ROW LEVEL SECURITY")
    op.execute(
        f"CREATE POLICY tenant_isolation ON {table} "
        f"USING ({column} = app_current_tenant()) "
        f"WITH CHECK ({column} = app_current_tenant())"
    )


def grant_app(table: str, privileges: str = "SELECT, INSERT, UPDATE, DELETE") -> None:
    op.execute(f"GRANT {privileges} ON {table} TO {APP_ROLE}")
