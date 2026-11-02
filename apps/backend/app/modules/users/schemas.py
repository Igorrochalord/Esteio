from datetime import datetime
from typing import Literal
from uuid import UUID

from pydantic import BaseModel, ConfigDict, EmailStr, Field

AssignableRole = Literal["admin", "user"]  # owner só nasce no onboarding


class UserOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: UUID
    name: str
    email: str
    role: str
    is_active: bool
    last_login_at: datetime | None
    created_at: datetime


class UserCreate(BaseModel):
    name: str = Field(min_length=2, max_length=200)
    email: EmailStr
    password: str = Field(min_length=8, max_length=128, description="Senha provisória")
    role: AssignableRole = "user"


class UserUpdate(BaseModel):
    name: str | None = Field(default=None, min_length=2, max_length=200)
    role: AssignableRole | None = None
    is_active: bool | None = None
