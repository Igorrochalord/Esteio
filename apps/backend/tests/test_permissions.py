import pytest

from tests.conftest import create_member, onboard, unique_email


@pytest.fixture
async def team(client):
    owner = await onboard(client)
    admin = await create_member(client, owner, "admin")
    user = await create_member(client, owner, "user")
    return owner, admin, user


async def test_plain_user_permissions(client, team):
    _, _, user = team
    assert (await client.get("/tenant/branding", headers=user.headers)).status_code == 200
    assert (await client.get("/me", headers=user.headers)).json()["permissions"] == {
        "settings": "read"
    }
    r = await client.put("/tenant/branding", headers=user.headers, json={"primary_color": "#000000"})
    assert r.status_code == 403
    assert (await client.get("/users", headers=user.headers)).status_code == 403
    assert (await client.get("/audit", headers=user.headers)).status_code == 403
    r = await client.post(
        "/users",
        headers=user.headers,
        json={"name": "Fulano", "email": unique_email(), "password": "senha12345"},
    )
    assert r.status_code == 403


async def test_admin_permissions(client, team):
    owner, admin, _ = team
    assert (await client.get("/users", headers=admin.headers)).status_code == 200
    assert (await client.get("/audit", headers=admin.headers)).status_code == 200
    r = await client.put("/tenant/branding", headers=admin.headers, json={"primary_color": "#123456"})
    assert r.status_code == 200 and r.json()["primary_color"] == "#123456"
    r = await client.post(
        "/users",
        headers=admin.headers,
        json={"name": "Novo", "email": unique_email(), "password": "senha12345", "role": "user"},
    )
    assert r.status_code == 201

    # admin não mexe no owner
    r = await client.patch(f"/users/{owner.user_id}", headers=admin.headers, json={"role": "user"})
    assert r.status_code == 422
    r = await client.patch(
        f"/users/{owner.user_id}", headers=admin.headers, json={"is_active": False}
    )
    assert r.status_code == 422


async def test_owner_role_cannot_be_assigned(client, team):
    owner, _, _ = team
    r = await client.post(
        "/users",
        headers=owner.headers,
        json={"name": "Fulano", "email": unique_email(), "password": "senha12345", "role": "owner"},
    )
    assert r.status_code == 422


async def test_cannot_change_own_access(client, team):
    owner, _, _ = team
    r = await client.patch(f"/users/{owner.user_id}", headers=owner.headers, json={"is_active": False})
    assert r.status_code == 422
    r = await client.patch(f"/users/{owner.user_id}", headers=owner.headers, json={"name": "Novo Nome"})
    assert r.status_code == 200 and r.json()["name"] == "Novo Nome"


async def test_role_change_is_audited(client, team):
    owner, admin, _ = team
    users = (await client.get("/users", headers=owner.headers)).json()
    admin_id = next(u["id"] for u in users if u["email"] == admin.email)
    r = await client.patch(f"/users/{admin_id}", headers=owner.headers, json={"role": "user"})
    assert r.status_code == 200 and r.json()["role"] == "user"
    page = (await client.get(f"/audit?entity=user&entity_id={admin_id}", headers=owner.headers)).json()
    upd = next(i for i in page["items"] if i["action"] == "user.update")
    assert upd["before"] == {"role": "admin"} and upd["after"] == {"role": "user"}

    # o novo papel vale já na próxima requisição
    assert (await client.get("/users", headers=admin.headers)).status_code == 403


async def test_duplicate_member_email(client, team):
    owner, admin, _ = team
    r = await client.post(
        "/users",
        headers=owner.headers,
        json={"name": "Fulano", "email": admin.email, "password": "senha12345"},
    )
    assert r.status_code == 409
