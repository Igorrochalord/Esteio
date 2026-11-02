from app.core import storage
from app.core.config import settings

PNG = b"\x89PNG\r\n\x1a\n" + b"\x00" * 64


async def test_branding_update_and_logo_lifecycle(client, tenant_a):
    r = await client.put(
        "/tenant/branding",
        headers=tenant_a.headers,
        json={"primary_color": "#112233", "secondary_color": "#445566"},
    )
    assert r.status_code == 200
    assert r.json()["primary_color"] == "#112233" and r.json()["logo_url"] is None

    r = await client.post(
        "/tenant/branding/logo", headers=tenant_a.headers, files={"file": ("logo.png", PNG)}
    )
    assert r.status_code == 200, r.text
    url = r.json()["logo_url"]
    assert url and url.startswith(settings.s3_public_endpoint_url or "")
    assert f"tenants/{tenant_a.tenant_id}/branding/" in url

    me = (await client.get("/me", headers=tenant_a.headers)).json()
    assert me["tenant"]["logo_url"]

    key = url.split(f"/{settings.s3_bucket}/", 1)[1].split("?", 1)[0]
    head = storage.internal_client().head_object(Bucket=settings.s3_bucket, Key=key)
    assert head["ContentType"] == "image/png"

    r = await client.delete("/tenant/branding/logo", headers=tenant_a.headers)
    assert r.status_code == 200 and r.json()["logo_url"] is None


async def test_logo_rejects_invalid_files(client, tenant_a):
    svg = b"<svg xmlns='http://www.w3.org/2000/svg'><script>alert(1)</script></svg>"
    r = await client.post(
        "/tenant/branding/logo", headers=tenant_a.headers, files={"file": ("x.svg", svg)}
    )
    assert r.status_code == 422
    big = PNG + b"\x00" * settings.logo_max_bytes
    r = await client.post(
        "/tenant/branding/logo", headers=tenant_a.headers, files={"file": ("big.png", big)}
    )
    assert r.status_code == 422


async def test_branding_rejects_invalid_color(client, tenant_a):
    r = await client.put(
        "/tenant/branding", headers=tenant_a.headers, json={"primary_color": "blue"}
    )
    assert r.status_code == 422


async def test_health(client):
    import httpx

    from app.main import app

    async with httpx.AsyncClient(transport=httpx.ASGITransport(app=app), base_url="http://t") as c:
        r = await c.get("/health")
    assert r.status_code == 200 and r.json()["checks"] == {"database": "ok", "storage": "ok"}
