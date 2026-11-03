//! Onboarding, login/sessão e matriz de permissões (substitui o teste de isolamento
//! entre tenants: aqui o que separa é o papel de cada usuário).

mod common;

use common::*;
use nucleo_core::modules::auth::service as auth;
use nucleo_core::modules::company::service as company;
use nucleo_core::modules::users::models::UserUpdate;
use nucleo_core::modules::users::service as users;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::CoreError;

#[tokio::test]
async fn first_run_onboarding_configures_and_logs_in_owner() {
    let (_dir, core) = setup().await;
    assert!(!company::status(&core).await.unwrap().configured);
    assert!(matches!(
        core.current().await,
        Err(CoreError::Unauthorized(_))
    ));

    let me = company::onboard(&core, onboarding_input(" Dono@Padaria.com.br "))
        .await
        .unwrap();
    assert_eq!(me.user.email, OWNER_EMAIL);
    assert_eq!(me.user.role, "owner");
    assert_eq!(me.permissions["backup"], "manage");
    assert_eq!(me.company.branding.primary_color, "#b45309");
    assert!(company::status(&core).await.unwrap().configured);

    let again = company::onboard(&core, onboarding_input("outro@x.com.br")).await;
    assert!(matches!(again, Err(CoreError::Conflict(_))));
}

#[tokio::test]
async fn onboarding_reports_invalid_fields() {
    let (_dir, core) = setup().await;
    let mut input = onboarding_input("sem-arroba");
    input.company.cnpj = Some("11.222.333/0001-82".into());
    input.owner.password = "curta".into();
    match company::onboard(&core, input).await {
        Err(CoreError::Validation(f)) => {
            assert!(f.contains_key("company.cnpj"));
            assert!(f.contains_key("owner.email"));
            assert!(f.contains_key("owner.password"));
        }
        other => panic!("esperava Validation, veio {other:?}"),
    }
    assert!(!company::status(&core).await.unwrap().configured);
}

#[tokio::test]
async fn login_rejects_bad_credentials_with_same_message() {
    let (_dir, core, _me) = onboarded().await;
    auth::logout(&core);
    for (email, pass) in [(OWNER_EMAIL, "errada-123"), ("ninguem@x.com.br", PASSWORD)] {
        match auth::login(&core, email, pass.into()).await {
            Err(CoreError::Unauthorized(m)) => assert_eq!(m, auth::INVALID_CREDENTIALS),
            other => panic!("esperava Unauthorized, veio {other:?}"),
        }
    }
    let me = login_as(&core, OWNER_EMAIL).await;
    assert!(me.user.last_login_at.is_some());
}

#[tokio::test]
async fn logout_ends_session() {
    let (_dir, core, _me) = onboarded().await;
    assert!(core.current().await.is_ok());
    auth::logout(&core);
    assert!(matches!(
        core.current().await,
        Err(CoreError::Unauthorized(_))
    ));
}

#[tokio::test]
async fn deactivating_a_user_cuts_access_immediately() {
    let (_dir, core, _me) = onboarded().await;
    let member = create_member(&core, "caixa@padaria.com.br", "user").await;
    login_as(&core, "caixa@padaria.com.br").await;
    assert!(core.current().await.is_ok());

    // outro "terminal" desativa o usuário direto no banco (mesmo efeito de um admin)
    sqlx::query("UPDATE users SET is_active = 0 WHERE id = ?")
        .bind(&member.id)
        .execute(&core.pool())
        .await
        .unwrap();
    assert!(matches!(
        core.current().await,
        Err(CoreError::Unauthorized(_))
    ));
    assert!(matches!(
        auth::login(&core, "caixa@padaria.com.br", PASSWORD.into()).await,
        Err(CoreError::Unauthorized(_))
    ));
}

#[tokio::test]
async fn role_matrix_is_enforced() {
    let (_dir, core, _me) = onboarded().await;
    create_member(&core, "admin@padaria.com.br", "admin").await;
    create_member(&core, "caixa@padaria.com.br", "user").await;

    login_as(&core, "caixa@padaria.com.br").await;
    assert!(core.require(Module::Settings, Action::Read).await.is_ok());
    for (m, a) in [
        (Module::Settings, Action::Manage),
        (Module::Users, Action::Read),
        (Module::Audit, Action::Read),
        (Module::Backup, Action::Write),
    ] {
        assert!(
            matches!(core.require(m, a).await, Err(CoreError::Forbidden(_))),
            "{m:?} {a:?}"
        );
    }

    login_as(&core, "admin@padaria.com.br").await;
    assert!(core.require(Module::Users, Action::Manage).await.is_ok());
    assert!(core.require(Module::Backup, Action::Write).await.is_ok());
    assert!(matches!(
        core.require(Module::Backup, Action::Manage).await,
        Err(CoreError::Forbidden(_))
    ));
}

#[tokio::test]
async fn user_rules_protect_owner_and_self() {
    let (_dir, core, me) = onboarded().await;
    let admin = create_member(&core, "admin@padaria.com.br", "admin").await;
    let caixa = create_member(&core, "caixa@padaria.com.br", "user").await;

    // dono não pode ser rebaixado nem desativado, nem mudar o próprio acesso
    let ctx = core.require(Module::Users, Action::Manage).await.unwrap();
    let demote = UserUpdate {
        role: Some("user".into()),
        ..Default::default()
    };
    assert!(matches!(
        users::update(&ctx, &me.user.id, demote).await,
        Err(CoreError::BusinessRule(_))
    ));

    // admin não edita o dono nem a si mesmo
    login_as(&core, "admin@padaria.com.br").await;
    let ctx = core.require(Module::Users, Action::Manage).await.unwrap();
    let rename = UserUpdate {
        name: Some("Outro Nome".into()),
        ..Default::default()
    };
    assert!(matches!(
        users::update(&ctx, &me.user.id, rename).await,
        Err(CoreError::BusinessRule(_))
    ));
    let self_off = UserUpdate {
        is_active: Some(false),
        ..Default::default()
    };
    assert!(matches!(
        users::update(&ctx, &admin.id, self_off).await,
        Err(CoreError::BusinessRule(_))
    ));

    // mas promove e desativa membros, com auditoria
    let promote = UserUpdate {
        role: Some("admin".into()),
        ..Default::default()
    };
    assert_eq!(
        users::update(&ctx, &caixa.id, promote).await.unwrap().role,
        "admin"
    );
    let off = UserUpdate {
        is_active: Some(false),
        ..Default::default()
    };
    assert!(!users::update(&ctx, &caixa.id, off).await.unwrap().is_active);

    let owner_role = UserUpdate {
        role: Some("owner".into()),
        ..Default::default()
    };
    assert!(matches!(
        users::update(&ctx, &caixa.id, owner_role).await,
        Err(CoreError::Validation(_))
    ));
}

#[tokio::test]
async fn duplicate_email_is_rejected_case_insensitively() {
    let (_dir, core, _me) = onboarded().await;
    create_member(&core, "caixa@padaria.com.br", "user").await;
    let ctx = core.require(Module::Users, Action::Manage).await.unwrap();
    let dup = nucleo_core::modules::users::models::UserCreate {
        name: "Outra".into(),
        email: "CAIXA@padaria.com.br".into(),
        password: PASSWORD.into(),
        role: "user".into(),
    };
    assert!(matches!(
        users::create(&ctx, dup).await,
        Err(CoreError::Conflict(_))
    ));
}
