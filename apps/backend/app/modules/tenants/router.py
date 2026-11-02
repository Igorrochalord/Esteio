from fastapi import APIRouter, Depends, Request, UploadFile, status
from sqlalchemy.ext.asyncio import AsyncSession

from app.core.config import settings
from app.core.context import RequestContext
from app.core.db import get_db
from app.core.deps import client_ip, get_context, require
from app.core.permissions import Action, Module
from app.modules.auth.schemas import TokenPair
from app.modules.tenants import service
from app.modules.tenants.schemas import (
    BrandingOut,
    BrandingUpdate,
    OnboardingRequest,
    TenantOut,
    TenantUpdate,
)

router = APIRouter(tags=["tenants"])


@router.post("/onboarding", response_model=TokenPair, status_code=status.HTTP_201_CREATED)
async def onboarding(
    data: OnboardingRequest, request: Request, db: AsyncSession = Depends(get_db)
) -> TokenPair:
    return await service.onboard(db, data, client_ip(request))


@router.get("/tenant", response_model=TenantOut)
async def get_tenant(ctx: RequestContext = Depends(require(Module.SETTINGS, Action.READ))):
    return await service.get_tenant(ctx)


@router.put("/tenant", response_model=TenantOut)
async def update_tenant(
    data: TenantUpdate, ctx: RequestContext = Depends(require(Module.SETTINGS, Action.MANAGE))
):
    return await service.update_tenant(ctx, data)


@router.get("/tenant/branding", response_model=BrandingOut)
async def get_branding(ctx: RequestContext = Depends(get_context)) -> BrandingOut:
    return await service.get_branding(ctx)


@router.put("/tenant/branding", response_model=BrandingOut)
async def update_branding(
    data: BrandingUpdate, ctx: RequestContext = Depends(require(Module.SETTINGS, Action.MANAGE))
) -> BrandingOut:
    return await service.update_branding(ctx, data)


@router.post("/tenant/branding/logo", response_model=BrandingOut)
async def upload_logo(
    file: UploadFile, ctx: RequestContext = Depends(require(Module.SETTINGS, Action.MANAGE))
) -> BrandingOut:
    content = await file.read(settings.logo_max_bytes + 1)
    return await service.upload_logo(ctx, content, settings.logo_max_bytes)


@router.delete("/tenant/branding/logo", response_model=BrandingOut)
async def remove_logo(
    ctx: RequestContext = Depends(require(Module.SETTINGS, Action.MANAGE)),
) -> BrandingOut:
    return await service.remove_logo(ctx)
