//! Planos comerciais: **hoje tudo está liberado** (acesso completo de lançamento).
//!
//! Este é o único lugar onde planos e limites vão morar. Quando a cobrança existir
//! (ver docs/assinatura.md), `access()` passa a ler a licença assinada e os services
//! chamam `ensure_within()` nos pontos marcados com `// PLANO:` no código.
//! Enquanto `access()` devolver `Access::Launch`, nenhuma checagem bloqueia nada.

use serde::Serialize;

use crate::error::{CoreError, CoreResult};

/// Planos previstos (valores mensais de referência; podem mudar até o lançamento da cobrança).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    /// grátis para sempre: gestão financeira, cadastros, comercial e estoque; sem NFS-e
    Gratuito,
    /// R$ 49/mês: + NFS-e (25/mês), OS ilimitada, contas bancárias ilimitadas, conciliação
    Starter,
    /// R$ 99/mês: + 100 NFS-e/mês, relatórios avançados (DRE), mais CNPJs
    Platinum,
    /// R$ 199/mês: + 1.000 NFS-e/mês, CNPJs ilimitados, suporte dedicado
    Infinite,
}

/// Limites de um plano. `None` = ilimitado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Limits {
    pub bank_accounts: Option<u32>,
    pub nfse_per_month: Option<u32>,
    /// CNPJs além do principal
    pub extra_cnpjs: Option<u32>,
    pub nfse: bool,
    pub service_orders: bool,
    pub bank_reconciliation: bool,
    pub advanced_reports: bool,
}

pub const fn limits(plan: Plan) -> Limits {
    match plan {
        Plan::Gratuito => Limits {
            bank_accounts: Some(3),
            nfse_per_month: Some(0),
            extra_cnpjs: Some(0),
            nfse: false,
            service_orders: false,
            bank_reconciliation: false,
            advanced_reports: false,
        },
        Plan::Starter => Limits {
            bank_accounts: None,
            nfse_per_month: Some(25),
            extra_cnpjs: Some(5),
            nfse: true,
            service_orders: true,
            bank_reconciliation: true,
            advanced_reports: false,
        },
        Plan::Platinum => Limits {
            bank_accounts: None,
            nfse_per_month: Some(100),
            extra_cnpjs: Some(10),
            nfse: true,
            service_orders: true,
            bank_reconciliation: true,
            advanced_reports: true,
        },
        Plan::Infinite => Limits {
            bank_accounts: None,
            nfse_per_month: Some(1000),
            extra_cnpjs: None,
            nfse: true,
            service_orders: true,
            bank_reconciliation: true,
            advanced_reports: true,
        },
    }
}

/// Situação de acesso desta instalação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// lançamento: tudo liberado, sem limites
    Launch,
    /// plano contratado (futuro: vem da licença assinada)
    Plan(Plan),
}

/// Hoje sempre `Launch`. PLANO: trocar pela leitura da licença quando a cobrança existir.
pub fn access() -> Access {
    Access::Launch
}

/// O que o app mostra ao usuário sobre o plano.
#[derive(Debug, Clone, Serialize)]
pub struct PlanInfo {
    /// "launch" ou o código do plano
    pub code: String,
    pub label: String,
    pub unlimited: bool,
    pub note: String,
}

pub fn info() -> PlanInfo {
    match access() {
        Access::Launch => PlanInfo {
            code: "launch".into(),
            label: "Acesso completo".into(),
            unlimited: true,
            note: "Todos os recursos estão liberados no lançamento. No futuro haverá planos \
                   (Gratuito, Starter, Platinum e Infinite); você será avisado antes de qualquer mudança."
                .into(),
        },
        Access::Plan(p) => PlanInfo {
            code: serde_json::to_value(p)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            label: format!("Plano {p:?}"),
            unlimited: false,
            note: String::new(),
        },
    }
}

/// Recurso sujeito a plano, para mensagens e checagens.
#[derive(Debug, Clone, Copy)]
pub enum Feature {
    BankAccounts,
    NfsePerMonth,
}

/// Barra o uso acima do limite do plano. Com `Access::Launch`, sempre permite.
pub fn ensure_within(feature: Feature, used: u32) -> CoreResult<()> {
    let Access::Plan(plan) = access() else {
        return Ok(());
    };
    let l = limits(plan);
    let (limit, what) = match feature {
        Feature::BankAccounts => (l.bank_accounts, "contas bancárias"),
        Feature::NfsePerMonth => (l.nfse_per_month, "NFS-e neste mês"),
    };
    match limit {
        Some(max) if used >= max => Err(CoreError::BusinessRule(format!(
            "Seu plano permite até {max} {what}. Faça upgrade para continuar."
        ))),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_is_unlimited() {
        assert_eq!(access(), Access::Launch);
        assert!(ensure_within(Feature::BankAccounts, 10_000).is_ok());
        assert!(info().unlimited);
    }

    #[test]
    fn planned_limits() {
        assert_eq!(limits(Plan::Gratuito).bank_accounts, Some(3));
        assert!(!limits(Plan::Gratuito).nfse);
        assert_eq!(limits(Plan::Starter).nfse_per_month, Some(25));
        assert_eq!(limits(Plan::Platinum).nfse_per_month, Some(100));
        assert_eq!(limits(Plan::Infinite).extra_cnpjs, None);
    }
}
