from pydantic import BaseModel, EmailStr

from app.modules.tenants.schemas import BrandingOut
from app.modules.users.schemas import UserOut


class LoginRequest(BaseModel):
    email: EmailStr
    password: str


class RefreshRequest(BaseModel):
    refresh_token: str


class TokenPair(BaseModel):
    access_token: str
    refresh_token: str
    token_type: str = "bearer"  # noqa: S105
    expires_in: int


class MeTenant(BrandingOut):
    id: str
    plan: str
    subscription_status: str


class MeOut(BaseModel):
    user: UserOut
    tenant: MeTenant
    permissions: dict[str, str]
