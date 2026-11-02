from fastapi import APIRouter, Depends, Request, Response, status
from sqlalchemy.ext.asyncio import AsyncSession

from app.core.context import RequestContext
from app.core.db import get_db
from app.core.deps import client_ip, get_context
from app.modules.auth import service
from app.modules.auth.schemas import LoginRequest, MeOut, RefreshRequest, TokenPair

router = APIRouter(tags=["auth"])


@router.post("/auth/login", response_model=TokenPair)
async def login(
    data: LoginRequest, request: Request, db: AsyncSession = Depends(get_db)
) -> TokenPair:
    return await service.login(db, data.email, data.password, client_ip(request))


@router.post("/auth/refresh", response_model=TokenPair)
async def refresh(data: RefreshRequest, db: AsyncSession = Depends(get_db)) -> TokenPair:
    return await service.refresh(db, data.refresh_token)


@router.post("/auth/logout", status_code=status.HTTP_204_NO_CONTENT)
async def logout(data: RefreshRequest, db: AsyncSession = Depends(get_db)) -> Response:
    await service.logout(db, data.refresh_token)
    return Response(status_code=status.HTTP_204_NO_CONTENT)


@router.get("/me", response_model=MeOut)
async def me(ctx: RequestContext = Depends(get_context)) -> MeOut:
    return await service.me(ctx)
