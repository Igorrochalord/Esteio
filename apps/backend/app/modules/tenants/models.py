from uuid import UUID, uuid4

from sqlalchemy import String
from sqlalchemy.orm import Mapped, mapped_column

from app.core.db import Base, TimestampMixin


class Tenant(TimestampMixin, Base):
    """Empresa cliente do SaaS. RLS: só enxerga a própria linha (id = tenant atual)."""

    __tablename__ = "tenants"

    id: Mapped[UUID] = mapped_column(primary_key=True, default=uuid4)
    name: Mapped[str] = mapped_column(String(200))
    cnpj: Mapped[str | None] = mapped_column(String(14))
    primary_color: Mapped[str] = mapped_column(String(7), default="#2563eb")
    secondary_color: Mapped[str] = mapped_column(String(7), default="#0f172a")
    logo_key: Mapped[str | None] = mapped_column(String(500))
    plan: Mapped[str] = mapped_column(String(30), default="trial")
    subscription_status: Mapped[str] = mapped_column(String(30), default="trialing")
