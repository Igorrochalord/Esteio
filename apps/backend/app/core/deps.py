from collections.abc import Awaitable, Callable

from fastapi import Depends, Request
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from sqlalchemy.ext.asyncio import AsyncSession

from app.core.context import RequestContext
from app.core.db import get_db, set_tenant
from app.core.errors import AuthenticationError, PermissionDeniedError
from app.core.permissions import Action, Module, is_allowed
from app.core.security import decode_token
from app.modules.users.models import User

_bearer = HTTPBearer(auto_error=False)


def client_ip(request: Request) -> str | None:
    return request.client.host if request.client else None


async def get_context(
    request: Request,
    db: AsyncSession = Depends(get_db),
    creds: HTTPAuthorizationCredentials | None = Depends(_bearer),
) -> RequestContext:
    if creds is None:
        raise AuthenticationError("Não autenticado")
    payload = decode_token(creds.credentials, "access")
    # a partir daqui toda query da transação está restrita ao tenant do token
    await set_tenant(db, payload["tid"])
    user = await db.get(User, payload["sub"])
    if user is None or not user.is_active:
        raise AuthenticationError("Usuário inválido ou desativado")
    return RequestContext(db=db, user=user, ip=client_ip(request))


def require(module: Module, action: Action) -> Callable[..., Awaitable[RequestContext]]:
    async def dependency(ctx: RequestContext = Depends(get_context)) -> RequestContext:
        if not is_allowed(ctx.user.role, module, action):
            raise PermissionDeniedError("Permissão insuficiente")
        return ctx

    return dependency
