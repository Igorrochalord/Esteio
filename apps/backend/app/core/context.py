from dataclasses import dataclass
from uuid import UUID

from sqlalchemy.ext.asyncio import AsyncSession

from app.modules.users.models import User


@dataclass(slots=True)
class RequestContext:
    """Tudo que um service precisa saber sobre a requisição autenticada.

    `db` já está numa transação com app.tenant_id definido.
    """

    db: AsyncSession
    user: User
    ip: str | None

    @property
    def tenant_id(self) -> UUID:
        return self.user.tenant_id
