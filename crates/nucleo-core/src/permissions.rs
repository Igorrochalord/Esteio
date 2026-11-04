//! RBAC: matriz estática papel → módulo → nível máximo.
//! `manage` implica `write`, que implica `read`. Módulo novo entra em `Module` e em `granted`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Owner,
    Admin,
    User,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::User => "user",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(Self::Owner),
            "admin" => Some(Self::Admin),
            "user" => Some(Self::User),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Module {
    /// dados e branding da empresa
    Settings,
    Users,
    Audit,
    /// cópias de segurança do banco
    Backup,
    /// catálogo de produtos, categorias e estoque
    Products,
    Customers,
    Sales,
    /// relatórios gerenciais (inclui custo, margem e desempenho de vendedores)
    Reports,
    /// contas bancárias, a pagar/receber, fluxo de caixa, DRE
    Finance,
    /// ordens de serviço
    ServiceOrders,
}

impl Module {
    pub const ALL: [Module; 10] = [
        Self::Settings,
        Self::Users,
        Self::Audit,
        Self::Backup,
        Self::Products,
        Self::Customers,
        Self::Sales,
        Self::Reports,
        Self::Finance,
        Self::ServiceOrders,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Settings => "settings",
            Self::Users => "users",
            Self::Audit => "audit",
            Self::Backup => "backup",
            Self::Products => "products",
            Self::Customers => "customers",
            Self::Sales => "sales",
            Self::Reports => "reports",
            Self::Finance => "finance",
            Self::ServiceOrders => "service_orders",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    Read = 1,
    Write = 2,
    Manage = 3,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Manage => "manage",
        }
    }
}

/// Nível máximo do papel no módulo (`None` = sem acesso).
pub fn granted(role: Role, module: Module) -> Option<Action> {
    use Action::*;
    use Module::*;
    match role {
        Role::Owner => Some(Manage),
        Role::Admin => match module {
            Settings | Users | Products | Customers | Sales | Finance | ServiceOrders => {
                Some(Manage)
            }
            Audit | Reports => Some(Read),
            Backup => Some(Write), // gera cópias; restaurar é só do dono
        },
        // operador do dia a dia: vende, cadastra clientes, consulta produtos
        Role::User => match module {
            Settings | Products => Some(Read),
            // técnico/atendente: abre, atualiza e fatura OS; cancelar é de quem gerencia
            Customers | Sales | ServiceOrders => Some(Write),
            _ => None,
        },
    }
}

pub fn is_allowed(role: Role, module: Module, action: Action) -> bool {
    granted(role, module).is_some_and(|g| g >= action)
}

/// Mapa módulo → nível, usado pelo frontend para montar o menu.
pub fn permissions_for(role: Role) -> BTreeMap<String, String> {
    Module::ALL
        .iter()
        .filter_map(|&m| granted(role, m).map(|a| (m.as_str().to_owned(), a.as_str().to_owned())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_cumulative() {
        assert!(is_allowed(Role::Admin, Module::Users, Action::Read));
        assert!(is_allowed(Role::Admin, Module::Backup, Action::Write));
        assert!(!is_allowed(Role::Admin, Module::Backup, Action::Manage));
        assert!(!is_allowed(Role::Admin, Module::Audit, Action::Write));
        assert!(!is_allowed(Role::User, Module::Users, Action::Read));
        assert!(is_allowed(Role::Owner, Module::Backup, Action::Manage));
    }

    #[test]
    fn user_menu_only_lists_granted_modules() {
        let p = permissions_for(Role::User);
        assert_eq!(p.len(), 5);
        assert_eq!(p["settings"], "read");
        assert_eq!(p["sales"], "write");
        assert!(!p.contains_key("reports"));
    }
}
