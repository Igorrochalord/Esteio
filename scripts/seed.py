"""Seed de desenvolvimento: duas empresas fictícias com usuários e branding.

Roda DENTRO do container da api (make seed), falando com a própria API via HTTP,
então passa pelas mesmas regras/RLS/auditoria que um cliente real. Idempotente.
"""

import struct
import sys
import zlib

import httpx

API = "http://127.0.0.1:8000/api/v1"
PASSWORD = "nucleo123"

COMPANIES = [
    {
        "company": {
            "name": "Padaria Pão Dourado",
            "cnpj": "11.222.333/0001-81",
            "primary_color": "#b45309",
            "secondary_color": "#451a03",
        },
        "owner": {"name": "Joana Dourado", "email": "dono@paodourado.com.br"},
        "members": [
            {"name": "Carlos Admin", "email": "admin@paodourado.com.br", "role": "admin"},
            {"name": "Bia Caixa", "email": "caixa@paodourado.com.br", "role": "user"},
        ],
    },
    {
        "company": {
            "name": "TechParts Distribuidora",
            "cnpj": None,
            "primary_color": "#0e7490",
            "secondary_color": "#082f49",
        },
        "owner": {"name": "Rafael Tech", "email": "dono@techparts.com.br"},
        "members": [
            {"name": "Lia Vendas", "email": "vendas@techparts.com.br", "role": "user"},
        ],
    },
]


def solid_png(hex_color: str, size: int = 64) -> bytes:
    """PNG de cor sólida, sem dependências (logo de exemplo)."""
    r, g, b = (int(hex_color[i : i + 2], 16) for i in (1, 3, 5))
    raw = b"".join(b"\x00" + bytes([r, g, b]) * size for _ in range(size))

    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data))

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )


def login(c: httpx.Client, email: str) -> str | None:
    r = c.post("/auth/login", json={"email": email, "password": PASSWORD})
    return r.json()["access_token"] if r.status_code == 200 else None


def seed_company(c: httpx.Client, spec: dict) -> None:
    owner = spec["owner"]
    token = login(c, owner["email"])
    if token:
        print(f"= {spec['company']['name']}: já existe")
    else:
        r = c.post(
            "/onboarding",
            json={"company": spec["company"], "owner": {**owner, "password": PASSWORD}},
        )
        r.raise_for_status()
        token = r.json()["access_token"]
        print(f"+ {spec['company']['name']}: criada")
    h = {"Authorization": f"Bearer {token}"}

    me = c.get("/me", headers=h).json()
    if not me["tenant"]["logo_url"]:
        png = solid_png(spec["company"]["primary_color"])
        c.post("/tenant/branding/logo", headers=h, files={"file": ("logo.png", png)}).raise_for_status()

    existing = {u["email"] for u in c.get("/users", headers=h).json()}
    for m in spec["members"]:
        if m["email"] in existing:
            continue
        c.post("/users", headers=h, json={**m, "password": PASSWORD}).raise_for_status()
        print(f"  + usuário {m['email']} ({m['role']})")


def main() -> int:
    with httpx.Client(base_url=API, timeout=15) as c:
        for spec in COMPANIES:
            seed_company(c, spec)
    print(f"\nSenha de todos os usuários do seed: {PASSWORD}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
