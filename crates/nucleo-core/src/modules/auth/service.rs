//! Login local. A sessão é só o id do usuário na memória do processo:
//! não há token salvo em disco, e fechar o app encerra a sessão.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::audit::{self, Entry};
use crate::db;
use crate::error::{CoreError, CoreResult};
use crate::modules::company::models::{Branding, Company};
use crate::modules::company::repository as company_repo;
use crate::modules::users::{models::User, repository as users_repo};
use crate::permissions;
use crate::security;
use crate::validate;
use crate::{Core, Ctx};

pub const INVALID_CREDENTIALS: &str = "E-mail ou senha inválidos";

#[derive(Debug, Serialize)]
pub struct MeCompany {
    pub id: String,
    #[serde(flatten)]
    pub branding: Branding,
}

#[derive(Debug, Serialize)]
pub struct Me {
    pub user: User,
    pub company: MeCompany,
    pub permissions: BTreeMap<String, String>,
}

pub(crate) fn me_of(user: User, company: &Company) -> Me {
    let permissions = permissions::permissions_for(user.role());
    Me {
        user,
        company: MeCompany {
            id: company.id.clone(),
            branding: company.branding(),
        },
        permissions,
    }
}

pub async fn login(core: &Core, email: &str, password: String) -> CoreResult<Me> {
    let email = validate::email(email).unwrap_or_default();
    let pool = core.pool();
    let mut tx = pool.begin().await?;
    let found = users_repo::find_by_email(&mut tx, &email).await?;
    // verifica a senha mesmo sem usuário (tempo constante)
    let password_ok =
        security::verify_password(password, found.as_ref().map(|u| u.password_hash.clone())).await;
    let mut user = match found {
        Some(u) if password_ok && u.is_active => u,
        _ => return Err(CoreError::Unauthorized(INVALID_CREDENTIALS.into())),
    };
    let company = company_repo::get(&mut tx)
        .await?
        .ok_or_else(|| CoreError::NotFound("Empresa não configurada".into()))?;

    let now = db::now();
    user.last_login_at = Some(now.clone());
    user.updated_at = now;
    users_repo::update(&mut tx, &user).await?;
    let entry = Entry {
        actor_id: Some(&user.id),
        action: "auth.login",
        entity: "user",
        entity_id: Some(&user.id),
        before: None,
        after: None,
    };
    audit::record(&mut tx, &core.installation(), entry).await?;
    tx.commit().await?;

    core.set_session(Some(user.id.clone()));
    Ok(me_of(user, &company))
}

pub fn logout(core: &Core) {
    core.set_session(None);
}

pub async fn me(ctx: &Ctx) -> CoreResult<Me> {
    let mut conn = ctx.db.acquire().await?;
    let company = company_repo::get(&mut conn)
        .await?
        .ok_or_else(|| CoreError::NotFound("Empresa não configurada".into()))?;
    Ok(me_of(ctx.user.clone(), &company))
}
