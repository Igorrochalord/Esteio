from uuid import UUID

from fastapi import APIRouter, Depends, status

from app.core.context import RequestContext
from app.core.deps import require
from app.core.permissions import Action, Module
from app.modules.users import service
from app.modules.users.schemas import UserCreate, UserOut, UserUpdate

router = APIRouter(prefix="/users", tags=["users"])


@router.get("", response_model=list[UserOut])
async def list_users(ctx: RequestContext = Depends(require(Module.USERS, Action.READ))):
    return await service.list_users(ctx)


@router.post("", response_model=UserOut, status_code=status.HTTP_201_CREATED)
async def create_user(
    data: UserCreate, ctx: RequestContext = Depends(require(Module.USERS, Action.MANAGE))
):
    return await service.create_user(ctx, data)


@router.get("/{user_id}", response_model=UserOut)
async def get_user(
    user_id: UUID, ctx: RequestContext = Depends(require(Module.USERS, Action.READ))
):
    return await service.get_user(ctx, user_id)


@router.patch("/{user_id}", response_model=UserOut)
async def update_user(
    user_id: UUID,
    data: UserUpdate,
    ctx: RequestContext = Depends(require(Module.USERS, Action.MANAGE)),
):
    return await service.update_user(ctx, user_id, data)
