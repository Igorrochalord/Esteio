use crate::audit::{self, Entry};
use crate::db;
use crate::error::{CoreError, CoreResult, Validator};
use crate::modules::company::repository as company_repo;
use crate::permissions::Role;
use crate::security;
use crate::validate;
use crate::{Core, Ctx};

use super::models::{RegisterInput, User, UserCreate, UserUpdate};
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
        pending: false,
        last_login_at: None,
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
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
    if touches_access && user.pending {
        return Err(CoreError::BusinessRule(
            "Este cadastro está aguardando aprovação: use Aprovar ou Recusar".into(),
        ));
    }
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

/// Autocadastro pela tela de login. A conta nasce inativa e aguardando aprovação:
/// quem está na frente do computador não ganha acesso sozinho.
pub async fn register(core: &Core, data: RegisterInput) -> CoreResult<()> {
    let mut v = Validator::default();
    let email = check_person(&mut v, "", &data.name, &data.email, &data.password);
    v.finish()?;
    let email = email.unwrap_or_default();

    let password_hash = security::hash_password(data.password).await?;
    let inst = core.installation();
    let pool = core.pool();
    let mut tx = pool.begin().await?;
    if !company_repo::exists(&mut tx).await? {
        return Err(CoreError::BusinessRule(
            "Configure a empresa antes de cadastrar pessoas".into(),
        ));
    }
    if repository::find_by_email(&mut tx, &email).await?.is_some() {
        return Err(CoreError::Conflict(EMAIL_TAKEN.into()));
    }
    let now = db::now();
    let user = User {
        id: db::new_id(),
        name: data.name.trim().to_owned(),
        email,
        password_hash,
        role: Role::User.as_str().into(),
        is_active: false,
        pending: true,
        last_login_at: None,
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
    };
    repository::insert(&mut tx, &inst, &user).await?;
    let entry = Entry {
        actor_id: Some(&user.id),
        action: "user.register",
        entity: "user",
        entity_id: Some(&user.id),
        before: None,
        after: Some(audit::snapshot(&user)),
    };
    audit::record(&mut tx, &inst, entry).await?;
    tx.commit().await?;
    Ok(())
}

async fn load_pending(conn: &mut db::Conn, user_id: &str) -> CoreResult<User> {
    let user = repository::get(conn, user_id)
        .await?
        .ok_or_else(|| CoreError::NotFound("Usuário não encontrado".into()))?;
    if !user.pending {
        return Err(CoreError::BusinessRule(
            "Este cadastro não está aguardando aprovação".into(),
        ));
    }
    Ok(user)
}

/// Grava a alteração e a auditoria (só os campos que mudaram).
async fn save(
    tx: &mut db::Conn,
    ctx: &Ctx,
    action: &str,
    before: serde_json::Value,
    user: &mut User,
) -> CoreResult<()> {
    user.updated_at = db::now();
    repository::update(tx, user).await?;
    let entry = Entry {
        actor_id: Some(&ctx.user.id),
        action,
        entity: "user",
        entity_id: Some(&user.id),
        before: Some(before),
        after: Some(audit::snapshot(&*user)),
    };
    audit::record_changes(tx, &ctx.inst, entry).await
}

/// Aprova um autocadastro com o papel escolhido.
pub async fn approve(ctx: &Ctx, user_id: &str, role: &str) -> CoreResult<User> {
    if !assignable(role) {
        return Err(CoreError::field("role", "Papel inválido"));
    }
    let mut tx = ctx.db.begin().await?;
    let mut user = load_pending(&mut tx, user_id).await?;
    let before = audit::snapshot(&user);
    user.pending = false;
    user.is_active = true;
    user.role = role.to_owned();
    save(&mut tx, ctx, "user.approve", before, &mut user).await?;
    tx.commit().await?;
    Ok(user)
}

/// Recusa um autocadastro (exclusão lógica: o e-mail fica livre para um novo pedido).
pub async fn reject(ctx: &Ctx, user_id: &str) -> CoreResult<()> {
    let mut tx = ctx.db.begin().await?;
    let mut user = load_pending(&mut tx, user_id).await?;
    let before = audit::snapshot(&user);
    user.deleted_at = Some(db::now());
    user.pending = false;
    save(&mut tx, ctx, "user.reject", before, &mut user).await?;
    tx.commit().await?;
    Ok(())
}

/// Define uma senha provisória para quem esqueceu a sua.
pub async fn reset_password(ctx: &Ctx, user_id: &str, password: String) -> CoreResult<()> {
    let n = password.chars().count();
    if !(8..=128).contains(&n) {
        return Err(CoreError::field(
            "password",
            "A senha precisa ter pelo menos 8 caracteres",
        ));
    }
    let mut tx = ctx.db.begin().await?;
    let mut user = repository::get(&mut tx, user_id)
        .await?
        .ok_or_else(|| CoreError::NotFound("Usuário não encontrado".into()))?;
    if user.id == ctx.user.id {
        return Err(CoreError::BusinessRule(
            "Para trocar a sua própria senha, peça a outro administrador".into(),
        ));
    }
    if user.role() == Role::Owner && ctx.user.role() != Role::Owner {
        return Err(CoreError::BusinessRule(
            "Apenas o dono pode redefinir a própria senha".into(),
        ));
    }
    user.password_hash = security::hash_password(password).await?;
    user.updated_at = db::now();
    repository::update(&mut tx, &user).await?;
    // a senha nunca vai para o histórico; registra só o fato
    let entry = Entry {
        actor_id: Some(&ctx.user.id),
        action: "user.password_reset",
        entity: "user",
        entity_id: Some(&user.id),
        before: None,
        after: None,
    };
    audit::record(&mut tx, &ctx.inst, entry).await?;
    tx.commit().await?;
    Ok(())
}
