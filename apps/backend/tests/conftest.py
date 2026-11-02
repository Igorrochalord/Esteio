"""Testes rodam contra o banco <db>_test, com a API conectando como usuário da
aplicação (NOBYPASSRLS) — então o isolamento testado é o da RLS de verdade."""

import os
import subprocess
import uuid
from collections.abc import AsyncIterator
from dataclasses import dataclass
from typing import Any

from sqlalchemy.engine import make_url


def _to_test_db(url: str) -> str:
    u = make_url(url)
    if not (u.database or "").endswith("_test"):
        u = u.set(database=f"{u.database}_test")
    return u.render_as_string(hide_password=False)


os.environ["DATABASE_URL"] = _to_test_db(os.environ["DATABASE_URL"])
os.environ["MIGRATIONS_DATABASE_URL"] = _to_test_db(os.environ["MIGRATIONS_DATABASE_URL"])
os.environ["DB_NULL_POOL"] = "true"

import httpx  # noqa: E402
import pytest  # noqa: E402

from app.main import app  # noqa: E402


def pytest_sessionstart(session: pytest.Session) -> None:
    # schema limpo a cada execução (e de quebra exercita o downgrade)
    for args in (["downgrade", "base"], ["upgrade", "head"]):
        subprocess.run(["alembic", *args], check=True, capture_output=True)


@pytest.fixture
async def client() -> AsyncIterator[httpx.AsyncClient]:
    transport = httpx.ASGITransport(app=app)
    async with httpx.AsyncClient(transport=transport, base_url="http://test/api/v1") as c:
        yield c


def unique_email(prefix: str = "user") -> str:
    return f"{prefix}-{uuid.uuid4().hex[:10]}@example.com"


def auth(token: str) -> dict[str, str]:
    return {"Authorization": f"Bearer {token}"}


@dataclass
class Account:
    email: str
    password: str
    access: str
    refresh: str
    user_id: str
    tenant_id: str

    @property
    def headers(self) -> dict[str, str]:
        return auth(self.access)


async def onboard(
    client: httpx.AsyncClient, company: str = "Empresa", **company_extra: Any
) -> Account:
    email, password = unique_email("owner"), "senha-forte-123"
    r = await client.post(
        "/onboarding",
        json={
            "company": {"name": company, **company_extra},
            "owner": {"name": "Dono", "email": email, "password": password},
        },
    )
    assert r.status_code == 201, r.text
    tokens = r.json()
    me = (await client.get("/me", headers=auth(tokens["access_token"]))).json()
    return Account(
        email,
        password,
        tokens["access_token"],
        tokens["refresh_token"],
        me["user"]["id"],
        me["tenant"]["id"],
    )


async def create_member(client: httpx.AsyncClient, owner: Account, role: str) -> Account:
    email, password = unique_email(role), "senha-forte-456"
    r = await client.post(
        "/users",
        headers=owner.headers,
        json={"name": f"Membro {role}", "email": email, "password": password, "role": role},
    )
    assert r.status_code == 201, r.text
    r = await client.post("/auth/login", json={"email": email, "password": password})
    assert r.status_code == 200, r.text
    t = r.json()
    return Account(email, password, t["access_token"], t["refresh_token"], "", owner.tenant_id)


@pytest.fixture
async def tenant_a(client: httpx.AsyncClient) -> Account:
    return await onboard(client, "Empresa A")


@pytest.fixture
async def tenant_b(client: httpx.AsyncClient) -> Account:
    return await onboard(client, "Empresa B")
