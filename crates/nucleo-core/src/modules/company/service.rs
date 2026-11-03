use base64::Engine;

use crate::audit::{self, Entry};
use crate::db;
use crate::error::{CoreError, CoreResult, Validator};
use crate::modules::auth::service::{self as auth, Me};
use crate::modules::users::{models::User, repository as users_repo, service as users};
use crate::permissions::Role;
use crate::security;
use crate::validate;
use crate::{Core, Ctx};

use super::models::{
    Branding, BrandingUpdate, Company, CompanyOut, CompanyUpdate, OnboardingInput, SetupStatus,
};
use super::repository;

pub const LOGO_MAX_BYTES: usize = 2 * 1024 * 1024;

/// assinatura → content-type. SVG fica de fora (pode conter script).
const LOGO_TYPES: [(&[u8], &str); 2] = [
    (b"\x89PNG\r\n\x1a\n", "image/png"),
    (b"\xff\xd8\xff", "image/jpeg"),
];

const CNPJ_INVALID: &str = "CNPJ inválido";

pub async fn status(core: &Core) -> CoreResult<SetupStatus> {
    let mut conn = core.pool().acquire().await?;
    Ok(SetupStatus {
        configured: repository::exists(&mut conn).await?,
    })
}

/// Primeira abertura: cria empresa + dono numa transação e já entra como dono.
pub async fn onboard(core: &Core, data: OnboardingInput) -> CoreResult<Me> {
    let mut v = Validator::default();
    let c = &data.company;
    v.check(
        validate::len_between(&c.name, 2, 200),
        "company.name",
        "Informe o nome da empresa (2 a 200 letras)",
    );
    let cnpj = validate::cnpj(c.cnpj.as_deref());
    v.check(cnpj.is_ok(), "company.cnpj", CNPJ_INVALID);
    v.check(
        validate::hex_color(&c.primary_color),
        "company.primary_color",
        "Cor inválida",
    );
    v.check(
        validate::hex_color(&c.secondary_color),
        "company.secondary_color",
        "Cor inválida",
    );
    let o = &data.owner;
    let email = users::check_person(&mut v, "owner.", &o.name, &o.email, &o.password);
    v.finish()?;

    let password_hash = security::hash_password(data.owner.password).await?;
    let inst = core.installation();
    let pool = core.pool();
    let mut tx = pool.begin().await?;
    if repository::exists(&mut tx).await? {
        return Err(CoreError::Conflict(
            "Este computador já tem uma empresa configurada".into(),
        ));
    }
    let now = db::now();
    let company = Company {
        id: db::new_id(),
        name: data.company.name.trim().to_owned(),
        cnpj: cnpj.unwrap_or_default(),
        primary_color: data.company.primary_color,
        secondary_color: data.company.secondary_color,
        logo: None,
        logo_type: None,
        logo_id: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let owner = User {
        id: db::new_id(),
        name: data.owner.name.trim().to_owned(),
        email: email.unwrap_or_default(),
        password_hash,
        role: Role::Owner.as_str().into(),
        is_active: true,
        last_login_at: Some(now.clone()),
        created_at: now.clone(),
        updated_at: now,
    };
    repository::insert(&mut tx, &inst, &company).await?;
    users_repo::insert(&mut tx, &inst, &owner).await?;
    for (action, entity, id, after) in [
        (
            "company.create",
            "company",
            &company.id,
            audit::snapshot(&company),
        ),
        ("user.create", "user", &owner.id, audit::snapshot(&owner)),
    ] {
        let entry = Entry {
            actor_id: Some(&owner.id),
            action,
            entity,
            entity_id: Some(id),
            before: None,
            after: Some(after),
        };
        audit::record(&mut tx, &inst, entry).await?;
    }
    tx.commit().await?;
    core.set_session(Some(owner.id.clone()));
    Ok(auth::me_of(owner, &company))
}

async fn load(conn: &mut db::Conn) -> CoreResult<Company> {
    repository::get(conn)
        .await?
        .ok_or_else(|| CoreError::NotFound("Empresa não configurada".into()))
}

pub async fn get(ctx: &Ctx) -> CoreResult<CompanyOut> {
    let mut conn = ctx.db.acquire().await?;
    Ok(CompanyOut::from(&load(&mut conn).await?))
}

/// Aplica a mudança, grava e audita (só se algo mudou). Base de todas as escritas da empresa.
async fn change(ctx: &Ctx, action: &str, apply: impl FnOnce(&mut Company)) -> CoreResult<Company> {
    let mut tx = ctx.db.begin().await?;
    let mut company = load(&mut tx).await?;
    let before = audit::snapshot(&company);
    apply(&mut company);
    let after = audit::snapshot(&company);
    if before != after {
        company.updated_at = db::now();
        repository::update(&mut tx, &company).await?;
        audit::record_changes(
            &mut tx,
            &ctx.inst,
            Entry {
                actor_id: Some(&ctx.user.id),
                action,
                entity: "company",
                entity_id: Some(&company.id),
                before: Some(before),
                after: Some(after),
            },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(company)
}

pub async fn update(ctx: &Ctx, data: CompanyUpdate) -> CoreResult<CompanyOut> {
    let mut v = Validator::default();
    if let Some(name) = &data.name {
        v.check(
            validate::len_between(name, 2, 200),
            "name",
            "Informe o nome da empresa (2 a 200 letras)",
        );
    }
    let cnpj = match &data.cnpj {
        Some(value) => {
            let parsed = validate::cnpj(value.as_deref());
            v.check(parsed.is_ok(), "cnpj", CNPJ_INVALID);
            Some(parsed.unwrap_or_default())
        }
        None => None,
    };
    v.finish()?;

    let company = change(ctx, "company.update", |c| {
        if let Some(name) = data.name {
            c.name = name.trim().to_owned();
        }
        if let Some(cnpj) = cnpj {
            c.cnpj = cnpj;
        }
    })
    .await?;
    Ok(CompanyOut::from(&company))
}

pub async fn update_branding(ctx: &Ctx, data: BrandingUpdate) -> CoreResult<Branding> {
    let mut v = Validator::default();
    for (field, value) in [
        ("primary_color", &data.primary_color),
        ("secondary_color", &data.secondary_color),
    ] {
        if let Some(color) = value {
            v.check(validate::hex_color(color), field, "Cor inválida");
        }
    }
    v.finish()?;

    let company = change(ctx, "branding.update", |c| {
        if let Some(p) = data.primary_color {
            c.primary_color = p;
        }
        if let Some(s) = data.secondary_color {
            c.secondary_color = s;
        }
    })
    .await?;
    Ok(company.branding())
}

/// `content` em base64 (aceita também o formato data URL).
pub async fn upload_logo(ctx: &Ctx, content: &str) -> CoreResult<Branding> {
    let b64 = content.split_once("base64,").map_or(content, |(_, b)| b);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| CoreError::BusinessRule("Não foi possível ler a imagem".into()))?;
    if bytes.len() > LOGO_MAX_BYTES {
        return Err(CoreError::BusinessRule(format!(
            "O logo pode ter no máximo {} KB",
            LOGO_MAX_BYTES / 1024
        )));
    }
    let content_type = LOGO_TYPES
        .iter()
        .find(|(sig, _)| bytes.starts_with(sig))
        .map(|(_, ty)| *ty)
        .ok_or_else(|| CoreError::BusinessRule("Formato inválido: envie PNG ou JPEG".into()))?;

    let company = change(ctx, "branding.logo_upload", |c| {
        c.logo = Some(bytes);
        c.logo_type = Some(content_type.into());
        c.logo_id = Some(db::new_id());
    })
    .await?;
    Ok(company.branding())
}

pub async fn remove_logo(ctx: &Ctx) -> CoreResult<Branding> {
    let company = change(ctx, "branding.logo_remove", |c| {
        c.logo = None;
        c.logo_type = None;
        c.logo_id = None;
    })
    .await?;
    Ok(company.branding())
}
