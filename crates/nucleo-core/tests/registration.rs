//! Autocadastro com aprovação, redefinição de senha e identidade visual pública.

mod common;

use common::*;
use nucleo_core::modules::auth::service as auth;
use nucleo_core::modules::company::service as company;
use nucleo_core::modules::users::models::{RegisterInput, UserUpdate};
use nucleo_core::modules::users::service as users;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::CoreError;

const NEW_EMAIL: &str = "novo@padaria.com.br";

fn register_input(email: &str) -> RegisterInput {
    RegisterInput {
        name: "Pessoa Nova".into(),
        email: email.into(),
        password: PASSWORD.into(),
    }
}

async fn pending_id(core: &nucleo_core::Core) -> String {
    let ctx = core.require(Module::Users, Action::Read).await.unwrap();
    users::list(&ctx)
        .await
        .unwrap()
        .into_iter()
        .find(|u| u.email == NEW_EMAIL)
        .expect("cadastro na lista")
        .id
}

#[tokio::test]
async fn registration_requires_a_configured_company() {
    let (_dir, core) = setup().await;
    let r = users::register(&core, register_input(NEW_EMAIL)).await;
    assert!(matches!(r, Err(CoreError::BusinessRule(_))));
}

#[tokio::test]
async fn registered_user_waits_for_approval_then_logs_in() {
    let (_dir, core, _me) = onboarded().await;
    auth::logout(&core);
    users::register(&core, register_input(" Novo@Padaria.com.br "))
        .await
        .unwrap();

    // mesma senha certa: avisa que está pendente; senha errada: mensagem genérica
    match auth::login(&core, NEW_EMAIL, PASSWORD.into()).await {
        Err(CoreError::Unauthorized(m)) => assert_eq!(m, auth::PENDING_APPROVAL),
        other => panic!("esperava pendente, veio {other:?}"),
    }
    match auth::login(&core, NEW_EMAIL, "errada-123".into()).await {
        Err(CoreError::Unauthorized(m)) => assert_eq!(m, auth::INVALID_CREDENTIALS),
        other => panic!("esperava inválido, veio {other:?}"),
    }

    // reativar pelo caminho comum não pula a aprovação
    login_as(&core, OWNER_EMAIL).await;
    let id = pending_id(&core).await;
    let ctx = core.require(Module::Users, Action::Manage).await.unwrap();
    let activate = UserUpdate {
        is_active: Some(true),
        ..Default::default()
    };
    assert!(matches!(
        users::update(&ctx, &id, activate).await,
        Err(CoreError::BusinessRule(_))
    ));

    let approved = users::approve(&ctx, &id, "admin").await.unwrap();
    assert!(approved.is_active && !approved.pending);
    assert_eq!(approved.role, "admin");
    assert!(matches!(
        users::approve(&ctx, &id, "user").await,
        Err(CoreError::BusinessRule(_))
    ));

    let me = login_as(&core, NEW_EMAIL).await;
    assert_eq!(me.permissions["users"], "manage");
}

#[tokio::test]
async fn rejected_registration_frees_the_email() {
    let (_dir, core, _me) = onboarded().await;
    users::register(&core, register_input(NEW_EMAIL))
        .await
        .unwrap();
    let dup = users::register(&core, register_input(NEW_EMAIL)).await;
    assert!(matches!(dup, Err(CoreError::Conflict(_))));

    let id = pending_id(&core).await;
    let ctx = core.require(Module::Users, Action::Manage).await.unwrap();
    users::reject(&ctx, &id).await.unwrap();
    let still_listed = users::list(&ctx)
        .await
        .unwrap()
        .iter()
        .any(|u| u.email == NEW_EMAIL);
    assert!(!still_listed);

    users::register(&core, register_input(NEW_EMAIL))
        .await
        .unwrap();
}

#[tokio::test]
async fn only_managers_approve() {
    let (_dir, core, _me) = onboarded().await;
    create_member(&core, "caixa@padaria.com.br", "user").await;
    users::register(&core, register_input(NEW_EMAIL))
        .await
        .unwrap();
    login_as(&core, "caixa@padaria.com.br").await;
    assert!(matches!(
        core.require(Module::Users, Action::Manage).await,
        Err(CoreError::Forbidden(_))
    ));
}

#[tokio::test]
async fn password_reset_rules() {
    let (_dir, core, me) = onboarded().await;
    let admin = create_member(&core, "admin@padaria.com.br", "admin").await;
    let caixa = create_member(&core, "caixa@padaria.com.br", "user").await;

    login_as(&core, "admin@padaria.com.br").await;
    let ctx = core.require(Module::Users, Action::Manage).await.unwrap();
    assert!(matches!(
        users::reset_password(&ctx, &caixa.id, "curta".into()).await,
        Err(CoreError::Validation(_))
    ));
    assert!(matches!(
        users::reset_password(&ctx, &me.user.id, "nova-senha-123".into()).await,
        Err(CoreError::BusinessRule(_))
    ));
    assert!(matches!(
        users::reset_password(&ctx, &admin.id, "nova-senha-123".into()).await,
        Err(CoreError::BusinessRule(_))
    ));
    users::reset_password(&ctx, &caixa.id, "nova-senha-123".into())
        .await
        .unwrap();

    auth::logout(&core);
    assert!(auth::login(&core, "caixa@padaria.com.br", PASSWORD.into())
        .await
        .is_err());
    auth::login(&core, "caixa@padaria.com.br", "nova-senha-123".into())
        .await
        .unwrap();

    // histórico registra o fato, nunca a senha
    login_as(&core, OWNER_EMAIL).await;
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs WHERE action = 'user.password_reset' AND after IS NULL",
    )
    .fetch_one(&core.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn public_branding_for_login_screen() {
    let (_dir, core) = setup().await;
    assert!(company::public_branding(&core).await.unwrap().is_none());
    company::onboard(&core, onboarding_input(OWNER_EMAIL))
        .await
        .unwrap();
    auth::logout(&core);
    let b = company::public_branding(&core).await.unwrap().unwrap();
    assert_eq!(b.name, "Padaria Pão Dourado");
    assert_eq!(b.primary_color, "#b45309");
}
