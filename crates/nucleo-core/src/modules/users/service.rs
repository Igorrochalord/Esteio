use crate::audit::{self, Entry};
use crate::db;
use crate::error::{CoreError, CoreResult, Validator};
use crate::permissions::Role;
use crate::security;
use crate::validate;
use crate::Ctx;

use super::models::{User, UserCreate, UserUpdate};
use super::repository;

pub const EMAIL_TAKEN: &str = "E-mail já cadastrado";

/// Papéis que podem ser atribuídos depois do onboarding.
fn assignable(role: &str) -> bool {
    matches!(Role::parse(role), Some(Role::Admin | Role::User))
}

/// Regras de nome/e-mail/senha compartilhadas com o onboarding (`prefix` = "owner." etc.).
pub(crate) fn check_person(
    v: &mut Validator,
    prefix: &str,
    name: &str,
    email: &str,
    password: &str,
) -> Option<String> {
    v.check(
        validate::len_between(name, 2, 200),
        &format!("{prefix}name"),
        "Informe o nome (2 a 200 letras)",
    );
    let email = validate::email(email);
    v.check(
        email.is_some(),
        &format!("{prefix}email"),
        "E-mail inválido",
    );
    let n = password.chars().count();
    v.check(
        (8..=128).contains(&n),
        &format!("{prefix}password"),
        "A senha precisa ter pelo menos 8 caracteres",
    );
    email
}

pub async fn list(ctx: &Ctx) -> CoreResult<Vec<User>> {
    let mut conn = ctx.db.acquire().await?;
    repository::list_all(&mut conn).await
}

pub async fn create(ctx: &Ctx, data: UserCreate) -> CoreResult<User> {
    let mut v = Validator::default();
    let email = check_person(&mut v, "", &data.name, &data.email, &data.password);
    v.check(assignable(&data.role), "role", "Papel inválido");
    v.finish()?;
    let email = email.unwrap_or_default();

    let password_hash = security::hash_password(data.password).await?;
    let mut tx = ctx.db.begin().await?;
    if repository::find_by_email(&mut tx, &email).await?.is_some() {
        return Err(CoreError::Conflict(EMAIL_TAKEN.into()));
    }
    let now = db::now();
    let user = User {
        id: db::new_id(),
        name: data.name.trim().to_owned(),
        email,
        password_hash,
        role: data.role,
        is_active: true,
        last_login_at: None,
        created_at: now.clone(),
        updated_at: now,
    };
    repository::insert(&mut tx, &ctx.inst, &user).await?;
    audit::record(
        &mut tx,
        &ctx.inst,
        Entry {
            actor_id: Some(&ctx.user.id),
            action: "user.create",
            entity: "user",
            entity_id: Some(&user.id),
            before: None,
            after: Some(audit::snapshot(&user)),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(user)
}

pub async fn update(ctx: &Ctx, user_id: &str, data: UserUpdate) -> CoreResult<User> {
    let mut v = Validator::default();
    if let Some(name) = &data.name {
        v.check(
            validate::len_between(name, 2, 200),
            "name",
            "Informe o nome (2 a 200 letras)",
        );
    }
    if let Some(role) = &data.role {
        v.check(assignable(role), "role", "Papel inválido");
    }
    v.finish()?;

    let mut tx = ctx.db.begin().await?;
    let mut user = repository::get(&mut tx, user_id)
        .await?
        .ok_or_else(|| CoreError::NotFound("Usuário não encontrado".into()))?;

    let touches_access = data.role.is_some() || data.is_active.is_some();
    if touches_access && user.role() == Role::Owner {
        return Err(CoreError::BusinessRule(
            "O dono não pode ser rebaixado nem desativado".into(),
        ));
    }
    if touches_access && user.id == ctx.user.id {
        return Err(CoreError::BusinessRule(
            "Você não pode alterar o próprio papel ou status".into(),
        ));
    }
    if user.role() == Role::Owner && ctx.user.role() != Role::Owner {
        return Err(CoreError::BusinessRule(
            "Apenas o dono pode editar os próprios dados".into(),
        ));
    }

    let before = audit::snapshot(&user);
    if let Some(name) = data.name {
        user.name = name.trim().to_owned();
    }
    if let Some(role) = data.role {
        user.role = role;
    }
    if let Some(active) = data.is_active {
        user.is_active = active;
    }
    let after = audit::snapshot(&user);
    let (b, a) = audit::diff(&before, &after);
    if !audit::is_empty(&a) {
        user.updated_at = db::now();
        repository::update(&mut tx, &user).await?;
        let action = if a.get("is_active") == Some(&serde_json::Value::Bool(false)) {
            "user.deactivate"
        } else {
            "user.update"
        };
        audit::record(
            &mut tx,
            &ctx.inst,
            Entry {
                actor_id: Some(&ctx.user.id),
                action,
                entity: "user",
                entity_id: Some(&user.id),
                before: Some(b),
                after: Some(a),
            },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(user)
}
