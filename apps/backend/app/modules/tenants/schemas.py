import re
from typing import Annotated

from pydantic import BaseModel, ConfigDict, EmailStr, Field, field_validator

HexColor = Annotated[str, Field(pattern=r"^#[0-9a-fA-F]{6}$", examples=["#2563eb"])]


def normalize_cnpj(value: str | None) -> str | None:
    if value is None or not value.strip():
        return None
    digits = re.sub(r"\D", "", value)
    if len(digits) != 14 or digits == digits[0] * 14:
        raise ValueError("CNPJ inválido")

    def check_digit(base: str, weights: list[int]) -> str:
        rest = sum(int(d) * w for d, w in zip(base, weights, strict=True)) % 11
        return "0" if rest < 2 else str(11 - rest)

    w1 = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]
    d1 = check_digit(digits[:12], w1)
    d2 = check_digit(digits[:12] + d1, [6, *w1])
    if digits[12:] != d1 + d2:
        raise ValueError("CNPJ inválido")
    return digits


class CompanyIn(BaseModel):
    name: str = Field(min_length=2, max_length=200)
    cnpj: str | None = None
    primary_color: HexColor = "#2563eb"
    secondary_color: HexColor = "#0f172a"

    _cnpj = field_validator("cnpj")(normalize_cnpj)


class OwnerIn(BaseModel):
    name: str = Field(min_length=2, max_length=200)
    email: EmailStr
    password: str = Field(min_length=8, max_length=128)


class OnboardingRequest(BaseModel):
    company: CompanyIn
    owner: OwnerIn


class BrandingOut(BaseModel):
    name: str
    primary_color: str
    secondary_color: str
    logo_url: str | None


class BrandingUpdate(BaseModel):
    primary_color: HexColor | None = None
    secondary_color: HexColor | None = None


class TenantOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: str
    name: str
    cnpj: str | None
    plan: str
    subscription_status: str

    @field_validator("id", mode="before")
    @classmethod
    def _uuid_str(cls, v: object) -> str:
        return str(v)


class TenantUpdate(BaseModel):
    name: str | None = Field(default=None, min_length=2, max_length=200)
    cnpj: str | None = None

    _cnpj = field_validator("cnpj")(normalize_cnpj)
