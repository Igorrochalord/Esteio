//! Helpers de teste: banco novo numa pasta temporária, onboarding e membros.
#![allow(dead_code)]

use nucleo_core::modules::auth::service::{self as auth, Me};
use nucleo_core::modules::company::models::{CompanyIn, OnboardingInput, OwnerIn};
use nucleo_core::modules::company::service as company;
use nucleo_core::modules::users::models::{User, UserCreate};
use nucleo_core::modules::users::service as users;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::Core;
use tempfile::TempDir;

pub const PASSWORD: &str = "senha-forte-123";
pub const OWNER_EMAIL: &str = "dono@padaria.com.br";

pub async fn setup() -> (TempDir, Core) {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = Core::open(dir.path()).await.expect("abrir core");
    (dir, core)
}

pub fn onboarding_input(email: &str) -> OnboardingInput {
    OnboardingInput {
        company: CompanyIn {
            name: "Padaria Pão Dourado".into(),
            cnpj: Some("11.222.333/0001-81".into()),
            primary_color: "#b45309".into(),
            secondary_color: "#451a03".into(),
        },
        owner: OwnerIn {
            name: "Joana Dourado".into(),
            email: email.into(),
            password: PASSWORD.into(),
        },
    }
}

/// Core já configurado e com o dono logado.
pub async fn onboarded() -> (TempDir, Core, Me) {
    let (dir, core) = setup().await;
    let me = company::onboard(&core, onboarding_input(OWNER_EMAIL))
        .await
        .expect("onboarding");
    (dir, core, me)
}

/// Cria um membro (logado como dono) e devolve-o. A sessão continua sendo a do dono.
pub async fn create_member(core: &Core, email: &str, role: &str) -> User {
    let ctx = core
        .require(Module::Users, Action::Manage)
        .await
        .expect("dono logado");
    users::create(
        &ctx,
        UserCreate {
            name: format!("Membro {role}"),
            email: email.into(),
            password: PASSWORD.into(),
            role: role.into(),
        },
    )
    .await
    .expect("criar membro")
}

pub async fn login_as(core: &Core, email: &str) -> Me {
    auth::login(core, email, PASSWORD.into())
        .await
        .expect("login")
}
