"""RBAC: matriz estática papel → módulo → ações.

`manage` implica `write`, que implica `read`. Novos módulos entram aqui
(inventory, sales, finance, contracts, reports nas próximas fases).
"""

from enum import StrEnum


class Role(StrEnum):
    OWNER = "owner"
    ADMIN = "admin"
    USER = "user"


class Module(StrEnum):
    SETTINGS = "settings"  # dados e branding da empresa
    USERS = "users"
    AUDIT = "audit"


class Action(StrEnum):
    READ = "read"
    WRITE = "write"
    MANAGE = "manage"


_LEVEL = {Action.READ: 1, Action.WRITE: 2, Action.MANAGE: 3}

# nível máximo de cada papel por módulo (ausente = sem acesso)
MATRIX: dict[Role, dict[Module, Action]] = {
    Role.OWNER: {m: Action.MANAGE for m in Module},
    Role.ADMIN: {
        Module.SETTINGS: Action.MANAGE,
        Module.USERS: Action.MANAGE,
        Module.AUDIT: Action.READ,
    },
    Role.USER: {
        Module.SETTINGS: Action.READ,
    },
}


def is_allowed(role: str, module: Module, action: Action) -> bool:
    granted = MATRIX.get(Role(role), {}).get(module)
    return granted is not None and _LEVEL[granted] >= _LEVEL[action]


def permissions_for(role: str) -> dict[str, str]:
    """Mapa módulo → nível, usado pelo frontend para montar o menu."""
    return {m.value: a.value for m, a in MATRIX.get(Role(role), {}).items()}
