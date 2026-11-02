import secrets
from datetime import UTC, datetime, timedelta
from typing import Any, Literal
from uuid import UUID, uuid4

import jwt
from argon2 import PasswordHasher
from argon2.exceptions import InvalidHashError, VerificationError

from app.core.config import settings
from app.core.errors import AuthenticationError

_hasher = PasswordHasher()
# hash fixo para gastar o mesmo tempo quando o e-mail não existe (evita enumeração por timing)
_DUMMY_HASH = _hasher.hash(secrets.token_urlsafe(16))

TokenType = Literal["access", "refresh"]


def hash_password(password: str) -> str:
    return _hasher.hash(password)


def verify_password(password: str, password_hash: str | None) -> bool:
    try:
        return _hasher.verify(password_hash or _DUMMY_HASH, password) and password_hash is not None
    except (VerificationError, InvalidHashError):
        return False


def _encode(claims: dict[str, Any], ttl: timedelta) -> tuple[str, datetime]:
    now = datetime.now(UTC)
    exp = now + ttl
    payload = {**claims, "iat": now, "exp": exp}
    return jwt.encode(payload, settings.jwt_secret, algorithm=settings.jwt_algorithm), exp


def create_access_token(user_id: UUID, tenant_id: UUID) -> str:
    token, _ = _encode(
        {"sub": str(user_id), "tid": str(tenant_id), "typ": "access"},
        timedelta(minutes=settings.access_token_ttl_minutes),
    )
    return token


def create_refresh_token(user_id: UUID, tenant_id: UUID) -> tuple[str, UUID, datetime]:
    jti = uuid4()
    token, exp = _encode(
        {"sub": str(user_id), "tid": str(tenant_id), "typ": "refresh", "jti": str(jti)},
        timedelta(days=settings.refresh_token_ttl_days),
    )
    return token, jti, exp


def decode_token(token: str, expected_type: TokenType) -> dict[str, Any]:
    try:
        payload: dict[str, Any] = jwt.decode(
            token,
            settings.jwt_secret,
            algorithms=[settings.jwt_algorithm],
            options={"require": ["exp", "sub", "tid", "typ"]},
        )
    except jwt.ExpiredSignatureError as exc:
        raise AuthenticationError("Token expirado") from exc
    except jwt.PyJWTError as exc:
        raise AuthenticationError("Token inválido") from exc
    if payload.get("typ") != expected_type:
        raise AuthenticationError("Tipo de token inválido")
    try:
        payload["sub"] = UUID(payload["sub"])
        payload["tid"] = UUID(payload["tid"])
        if "jti" in payload:
            payload["jti"] = UUID(payload["jti"])
    except ValueError as exc:
        raise AuthenticationError("Token inválido") from exc
    return payload
