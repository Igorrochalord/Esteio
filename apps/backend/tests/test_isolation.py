"""Isolamento entre empresas: pela API e direto no banco (RLS)."""

import uuid

import pytest
from sqlalchemy import text
from sqlalchemy.exc import DBAPIError

from app.core.db import engine
from tests.conftest import create_member


async def test_users_are_invisible_across_tenants(client, tenant_a, tenant_b):
    member_a = await create_member(client, tenant_a, "user")
    users_a = (await client.get("/users", headers=tenant_a.headers)).json()
    member_id = next(u["id"] for u in users_a if u["email"] == member_a.email)

    users_b = (await client.get("/users", headers=tenant_b.headers)).json()
    assert {u["email"] for u in users_b} == {tenant_b.email}

    # mesmo conhecendo o id, B não lê nem altera usuário de A
    assert (await client.get(f"/users/{member_id}", headers=tenant_b.headers)).status_code == 404
    r = await client.patch(
        f"/users/{member_id}", headers=tenant_b.headers, json={"is_active": False}
    )
    assert r.status_code == 404
    r = await client.get(f"/users/{member_id}", headers=tenant_a.headers)
    assert r.json()["is_active"] is True


async def test_tenant_data_and_audit_are_scoped(client, tenant_a, tenant_b):
    await client.put(
        "/tenant/branding", headers=tenant_a.headers, json={"primary_color": "#aa0000"}
    )
    b = (await client.get("/tenant/branding", headers=tenant_b.headers)).json()
    assert b["name"] == "Empresa B" and b["primary_color"] != "#aa0000"
    assert (await client.get("/tenant", headers=tenant_b.headers)).json()[
        "id"
    ] == tenant_b.tenant_id

    audit_b = (await client.get("/audit?limit=200", headers=tenant_b.headers)).json()["items"]
    assert audit_b and all(i["user_id"] != tenant_a.user_id for i in audit_b)


async def test_refresh_token_of_other_tenant_cannot_be_forged(client, tenant_a, tenant_b):
    # o refresh de A vale só para A; mesmo trocando contexto não vira sessão de B
    r = await client.post("/auth/refresh", json={"refresh_token": tenant_a.refresh})
    me = await client.get("/me", headers={"Authorization": f"Bearer {r.json()['access_token']}"})
    assert me.json()["tenant"]["id"] == tenant_a.tenant_id


# ---------- direto no banco, como o usuário da aplicação ----------


async def test_app_role_cannot_bypass_rls():
    async with engine.connect() as conn:
        row = (
            await conn.execute(
                text("SELECT rolbypassrls, rolsuper FROM pg_roles WHERE rolname = current_user")
            )
        ).one()
        assert row == (False, False)


async def test_without_tenant_context_nothing_is_visible(tenant_a, tenant_b):
    async with engine.connect() as conn:
        for table in ("tenants", "users", "refresh_tokens", "audit_logs"):
            count = (await conn.execute(text(f"SELECT count(*) FROM {table}"))).scalar_one()  # noqa: S608
            assert count == 0, table


async def test_tenant_context_limits_rows(tenant_a, tenant_b):
    async with engine.begin() as conn:
        await conn.execute(
            text("SELECT set_config('app.tenant_id', :t, true)"), {"t": tenant_a.tenant_id}
        )
        tenant_ids = (
            (await conn.execute(text("SELECT DISTINCT tenant_id::text FROM users"))).scalars().all()
        )
        assert tenant_ids == [tenant_a.tenant_id]
        tenants = (await conn.execute(text("SELECT id::text FROM tenants"))).scalars().all()
        assert tenants == [tenant_a.tenant_id]


async def test_cannot_write_into_other_tenant(tenant_a, tenant_b):
    async with engine.begin() as conn:
        await conn.execute(
            text("SELECT set_config('app.tenant_id', :t, true)"), {"t": tenant_a.tenant_id}
        )
        # UPDATE em linhas de B não afeta nada (invisíveis)
        res = await conn.execute(
            text("UPDATE users SET name = 'hack' WHERE tenant_id = :b"), {"b": tenant_b.tenant_id}
        )
        assert res.rowcount == 0

    with pytest.raises(DBAPIError, match="row-level security"):
        async with engine.begin() as conn:
            await conn.execute(
                text("SELECT set_config('app.tenant_id', :t, true)"), {"t": tenant_a.tenant_id}
            )
            await conn.execute(
                text(
                    "INSERT INTO users (id, tenant_id, name, email, password_hash, role, is_active) "
                    "VALUES (:id, :b, 'x', :e, 'x', 'user', true)"
                ),
                {"id": uuid.uuid4(), "b": tenant_b.tenant_id, "e": f"{uuid.uuid4().hex}@x.com"},
            )


async def test_audit_log_is_append_only(tenant_a):
    for stmt in ("UPDATE audit_logs SET action = 'x'", "DELETE FROM audit_logs"):
        with pytest.raises(DBAPIError, match="permission denied"):
            async with engine.begin() as conn:
                await conn.execute(
                    text("SELECT set_config('app.tenant_id', :t, true)"), {"t": tenant_a.tenant_id}
                )
                await conn.execute(text(stmt))
