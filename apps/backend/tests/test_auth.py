from tests.conftest import auth, create_member, onboard


async def test_login_success_and_failures(client):
    acc = await onboard(client)
    ok = await client.post(
        "/auth/login", json={"email": acc.email.upper(), "password": acc.password}
    )
    assert ok.status_code == 200
    assert (await client.get("/me", headers=auth(ok.json()["access_token"]))).status_code == 200

    bad = await client.post("/auth/login", json={"email": acc.email, "password": "errada123"})
    assert bad.status_code == 401
    unknown = await client.post(
        "/auth/login", json={"email": "ninguem@example.com", "password": "x"}
    )
    assert unknown.status_code == 401
    assert bad.json()["detail"] == unknown.json()["detail"]  # não revela se o e-mail existe


async def test_requires_token(client):
    assert (await client.get("/me")).status_code == 401
    assert (await client.get("/me", headers=auth("lixo"))).status_code == 401


async def test_refresh_rotates_and_detects_reuse(client):
    acc = await onboard(client)
    r1 = await client.post("/auth/refresh", json={"refresh_token": acc.refresh})
    assert r1.status_code == 200
    new = r1.json()
    assert new["refresh_token"] != acc.refresh
    assert (await client.get("/me", headers=auth(new["access_token"]))).status_code == 200

    # reusar o refresh antigo => 401 e TODAS as sessões do usuário são revogadas
    reuse = await client.post("/auth/refresh", json={"refresh_token": acc.refresh})
    assert reuse.status_code == 401
    r2 = await client.post("/auth/refresh", json={"refresh_token": new["refresh_token"]})
    assert r2.status_code == 401


async def test_access_token_cannot_be_used_as_refresh(client):
    acc = await onboard(client)
    r = await client.post("/auth/refresh", json={"refresh_token": acc.access})
    assert r.status_code == 401
    r = await client.get("/me", headers=auth(acc.refresh))
    assert r.status_code == 401


async def test_logout_revokes_refresh(client):
    acc = await onboard(client)
    assert (
        await client.post("/auth/logout", json={"refresh_token": acc.refresh})
    ).status_code == 204
    r = await client.post("/auth/refresh", json={"refresh_token": acc.refresh})
    assert r.status_code == 401


async def test_deactivated_user_loses_access_immediately(client):
    owner = await onboard(client)
    member = await create_member(client, owner, "user")
    users = (await client.get("/users", headers=owner.headers)).json()
    member_id = next(u["id"] for u in users if u["email"] == member.email)

    r = await client.patch(f"/users/{member_id}", headers=owner.headers, json={"is_active": False})
    assert r.status_code == 200 and r.json()["is_active"] is False

    assert (await client.get("/me", headers=member.headers)).status_code == 401
    r = await client.post("/auth/refresh", json={"refresh_token": member.refresh})
    assert r.status_code == 401
    r = await client.post("/auth/login", json={"email": member.email, "password": member.password})
    assert r.status_code == 401
