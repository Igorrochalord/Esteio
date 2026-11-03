//! Empresa (dados, cores, logo), histórico e backup/restauração.

mod common;

use base64::Engine;
use common::*;
use nucleo_core::modules::audit::models::AuditQuery;
use nucleo_core::modules::audit::service as audit;
use nucleo_core::modules::backup::service as backup;
use nucleo_core::modules::company::models::{BrandingUpdate, CompanyUpdate};
use nucleo_core::modules::company::service as company;
use nucleo_core::permissions::{Action, Module};
use nucleo_core::CoreError;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR";

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn query(entity: Option<&str>) -> AuditQuery {
    AuditQuery {
        limit: 50,
        offset: 0,
        entity: entity.map(Into::into),
        entity_id: None,
    }
}

#[tokio::test]
async fn company_update_validates_and_audits_only_changes() {
    let (_dir, core, _me) = onboarded().await;
    let ctx = core
        .require(Module::Settings, Action::Manage)
        .await
        .unwrap();

    let bad = CompanyUpdate {
        name: None,
        cnpj: Some(Some("123".into())),
    };
    assert!(matches!(
        company::update(&ctx, bad).await,
        Err(CoreError::Validation(_))
    ));

    let out = company::update(
        &ctx,
        CompanyUpdate {
            name: Some("  Pão Dourado  ".into()),
            cnpj: Some(None),
        },
    )
    .await
    .unwrap();
    assert_eq!(out.name, "Pão Dourado");
    assert_eq!(out.cnpj, None);

    // mesma coisa de novo: nada muda, nada é auditado
    let before = audit::list(&ctx, query(Some("company")))
        .await
        .unwrap()
        .total;
    company::update(
        &ctx,
        CompanyUpdate {
            name: Some("Pão Dourado".into()),
            cnpj: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        audit::list(&ctx, query(Some("company")))
            .await
            .unwrap()
            .total,
        before
    );

    let page = audit::list(&ctx, query(Some("company"))).await.unwrap();
    let last = &page.items[0];
    assert_eq!(last.action, "company.update");
    assert_eq!(last.user_name.as_deref(), Some("Joana Dourado"));
    assert_eq!(last.after.as_ref().unwrap()["name"], "Pão Dourado");
    assert!(last.after.as_ref().unwrap().get("updated_at").is_none());
}

#[tokio::test]
async fn branding_and_logo_rules() {
    let (_dir, core, _me) = onboarded().await;
    let ctx = core
        .require(Module::Settings, Action::Manage)
        .await
        .unwrap();

    let bad = BrandingUpdate {
        primary_color: Some("azul".into()),
        secondary_color: None,
    };
    assert!(matches!(
        company::update_branding(&ctx, bad).await,
        Err(CoreError::Validation(_))
    ));
    let ok = BrandingUpdate {
        primary_color: Some("#0e7490".into()),
        secondary_color: None,
    };
    assert_eq!(
        company::update_branding(&ctx, ok)
            .await
            .unwrap()
            .primary_color,
        "#0e7490"
    );

    let svg = b64(b"<svg onload=alert(1)></svg>");
    assert!(matches!(
        company::upload_logo(&ctx, &svg).await,
        Err(CoreError::BusinessRule(_))
    ));
    let huge = b64(&[PNG, &vec![0u8; company::LOGO_MAX_BYTES]].concat());
    assert!(matches!(
        company::upload_logo(&ctx, &huge).await,
        Err(CoreError::BusinessRule(_))
    ));

    let data_url = format!("data:image/png;base64,{}", b64(PNG));
    let branding = company::upload_logo(&ctx, &data_url).await.unwrap();
    assert!(branding
        .logo_url
        .unwrap()
        .starts_with("data:image/png;base64,"));
    // segundo envio do mesmo arquivo também conta como troca (logo_id novo)
    company::upload_logo(&ctx, &b64(PNG)).await.unwrap();
    assert!(company::remove_logo(&ctx).await.unwrap().logo_url.is_none());

    let actions: Vec<String> = audit::list(&ctx, query(Some("company")))
        .await
        .unwrap()
        .items
        .into_iter()
        .map(|e| e.action)
        .collect();
    assert_eq!(
        actions,
        [
            "branding.logo_remove",
            "branding.logo_upload",
            "branding.logo_upload",
            "branding.update",
            "company.create"
        ]
    );
}

#[tokio::test]
async fn audit_pagination() {
    let (_dir, core, _me) = onboarded().await;
    for i in 0..3 {
        create_member(&core, &format!("m{i}@padaria.com.br"), "user").await;
    }
    let ctx = core.require(Module::Audit, Action::Read).await.unwrap();
    let all = audit::list(&ctx, query(None)).await.unwrap();
    assert_eq!(all.total, 5); // company.create + 4 user.create
    let page = audit::list(
        &ctx,
        AuditQuery {
            limit: 2,
            offset: 4,
            entity: None,
            entity_id: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].action, "company.create");
}

#[tokio::test]
async fn backup_and_restore_roundtrip() {
    let (dir, core, _me) = onboarded().await;
    let ctx = core.require(Module::Backup, Action::Manage).await.unwrap();

    let snap = backup::create(&core, &ctx, None).await.unwrap();
    assert_eq!(snap.kind, "manual");
    assert!(snap.size_bytes > 0);

    // depois do backup: cria um usuário que NÃO está na cópia
    create_member(&core, "depois@padaria.com.br", "user").await;

    let safety = backup::restore(&core, &ctx, std::path::Path::new(&snap.path))
        .await
        .unwrap();
    assert_eq!(safety.kind, "pre-restore");
    // sessão encerrada: os usuários podem ser outros
    assert!(matches!(
        core.current().await,
        Err(CoreError::Unauthorized(_))
    ));

    login_as(&core, OWNER_EMAIL).await;
    let ctx = core.require(Module::Users, Action::Read).await.unwrap();
    let emails: Vec<String> = nucleo_core::modules::users::service::list(&ctx)
        .await
        .unwrap()
        .into_iter()
        .map(|u| u.email)
        .collect();
    assert_eq!(emails, [OWNER_EMAIL]);

    let ctx = core.require(Module::Audit, Action::Read).await.unwrap();
    let restores = audit::list(&ctx, query(Some("backup"))).await.unwrap();
    assert_eq!(restores.items[0].action, "backup.restore");

    let kinds: Vec<String> = backup::list(&core)
        .unwrap()
        .into_iter()
        .map(|b| b.kind)
        .collect();
    assert!(kinds.contains(&"manual".into()) && kinds.contains(&"pre-restore".into()));

    // a cópia de segurança guarda o estado anterior (com o usuário criado depois)
    let _ = dir;
    let content = std::fs::read(&safety.path).unwrap();
    assert!(content
        .windows(b"depois@padaria".len())
        .any(|w| w == b"depois@padaria"));
}

#[tokio::test]
async fn restore_rejects_invalid_files_and_keeps_data() {
    let (dir, core, _me) = onboarded().await;
    let ctx = core.require(Module::Backup, Action::Manage).await.unwrap();
    let junk = dir.path().join("planilha.db");
    std::fs::write(&junk, b"isto nao e um banco").unwrap();
    assert!(matches!(
        backup::restore(&core, &ctx, &junk).await,
        Err(CoreError::BusinessRule(_))
    ));

    // SQLite válido, mas que não é do Núcleo ERP
    let foreign = dir.path().join("outro-sistema.db");
    let url = format!("sqlite://{}?mode=rwc", foreign.display());
    let mut conn = <sqlx::SqliteConnection as sqlx::Connection>::connect(&url)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE clientes (id INTEGER)")
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::Connection::close(conn).await.unwrap();
    assert!(matches!(
        backup::restore(&core, &ctx, &foreign).await,
        Err(CoreError::BusinessRule(_))
    ));

    // sessão e dados seguem intactos
    assert!(core.current().await.is_ok());
    assert!(company::status(&core).await.unwrap().configured);
}

#[tokio::test]
async fn auto_backup_runs_once_per_day_and_rotates() {
    let (_dir, core, _me) = onboarded().await;
    let first = backup::auto_daily(&core)
        .await
        .unwrap()
        .expect("primeiro backup automático");
    assert!(
        backup::auto_daily(&core).await.unwrap().is_none(),
        "só um por dia"
    );

    // 10 backups automáticos antigos (> 24 h) e nenhum recente
    std::fs::remove_file(&first.path).unwrap();
    for i in 0..10u64 {
        let p = core
            .backups_dir()
            .join(format!("auto-2020010{i}-000000-000.db"));
        std::fs::write(&p, b"x").unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(86_400 * (30 - i));
        std::fs::File::options()
            .write(true)
            .open(&p)
            .unwrap()
            .set_modified(old)
            .unwrap();
    }
    assert!(backup::auto_daily(&core).await.unwrap().is_some());
    let autos = backup::list(&core)
        .unwrap()
        .into_iter()
        .filter(|b| b.kind == "auto")
        .count();
    assert_eq!(autos, backup::KEEP_AUTO);
}
