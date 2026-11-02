from tests.conftest import auth, onboard, unique_email


async def test_onboarding_creates_company_and_owner(client):
    email = unique_email("ana")
    r = await client.post(
        "/onboarding",
        json={
            "company": {
                "name": "Acme Ltda",
                "cnpj": "11.222.333/0001-81",
                "primary_color": "#ff0000",
            },
            "owner": {"name": "Ana", "email": email.upper(), "password": "senha12345"},
        },
    )
    assert r.status_code == 201, r.text
    body = r.json()
    assert body["token_type"] == "bearer" and body["access_token"] and body["refresh_token"]

    me = (await client.get("/me", headers=auth(body["access_token"]))).json()
    assert me["user"]["email"] == email  # normalizado em minúsculas
    assert me["user"]["role"] == "owner"
    assert me["tenant"]["name"] == "Acme Ltda"
    assert me["tenant"]["primary_color"] == "#ff0000"
    assert me["tenant"]["plan"] == "trial"
    assert me["permissions"]["users"] == "manage"

    tenant = (await client.get("/tenant", headers=auth(body["access_token"]))).json()
    assert tenant["cnpj"] == "11222333000181"


async def test_onboarding_writes_audit(client):
    acc = await onboard(client)
    items = (await client.get("/audit", headers=acc.headers)).json()["items"]
    actions = {i["action"] for i in items}
    assert {"tenant.create", "user.create"} <= actions
    user_create = next(i for i in items if i["action"] == "user.create")
    assert "password_hash" not in user_create["after"]


async def test_onboarding_duplicate_email_is_conflict(client):
    acc = await onboard(client)
    r = await client.post(
        "/onboarding",
        json={
            "company": {"name": "Outra"},
            "owner": {"name": "Fulano", "email": acc.email.upper(), "password": "senha12345"},
        },
    )
    assert r.status_code == 409


async def test_onboarding_validation(client):
    base = {"owner": {"name": "Fulano", "email": unique_email(), "password": "senha12345"}}
    r = await client.post("/onboarding", json={**base, "company": {"name": "A", "cnpj": "123"}})
    assert r.status_code == 422
    r = await client.post(
        "/onboarding", json={**base, "company": {"name": "Acme", "cnpj": "11.222.333/0001-82"}}
    )
    assert r.status_code == 422  # dígito verificador errado
    r = await client.post(
        "/onboarding", json={**base, "company": {"name": "Acme", "primary_color": "red"}}
    )
    assert r.status_code == 422
    short = {"name": "Fulano", "email": unique_email(), "password": "123"}
    r = await client.post("/onboarding", json={"company": {"name": "Acme"}, "owner": short})
    assert r.status_code == 422
