//! Garantias estruturais do banco: colunas de sync, change_log, auditoria imutável.

mod common;

use common::*;
use nucleo_core::db;

/// Tabelas de infraestrutura; todas as outras são de negócio e seguem as regras de sync.
const INFRA: &[&str] = &[
    "installation",
    "change_log",
    "audit_logs",
    "_sqlx_migrations",
    "sqlite_sequence",
];
const SYNC_COLUMNS: &[&str] = &[
    "id",
    "branch_id",
    "device_id",
    "created_at",
    "updated_at",
    "deleted_at",
];

async fn business_tables(core: &nucleo_core::Core) -> Vec<String> {
    let tables: Vec<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table'")
            .fetch_all(&core.pool())
            .await
            .unwrap();
    tables
        .into_iter()
        .filter(|t| !INFRA.contains(&t.as_str()))
        .collect()
}

#[tokio::test]
async fn every_business_table_is_sync_ready() {
    let (_dir, core) = setup().await;
    let tables = business_tables(&core).await;
    assert!(tables.contains(&"users".to_string()) && tables.contains(&"company".to_string()));

    for table in tables {
        let cols: Vec<String> =
            sqlx::query_scalar(&format!("SELECT name FROM pragma_table_info('{table}')"))
                .fetch_all(&core.pool())
                .await
                .unwrap();
        for c in SYNC_COLUMNS {
            assert!(cols.iter().any(|x| x == c), "{table} sem coluna {c}");
        }
        let triggers: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'trigger' AND tbl_name = ?",
        )
        .bind(&table)
        .fetch_all(&core.pool())
        .await
        .unwrap();
        for suffix in ["_cl_insert", "_cl_update", "_no_delete"] {
            assert!(
                triggers.iter().any(|t| t.ends_with(suffix)),
                "{table} sem trigger *{suffix} (change_log / exclusão lógica)"
            );
        }
    }
}

#[tokio::test]
async fn writes_feed_change_log_and_hard_delete_is_blocked() {
    let (_dir, core, me) = onboarded().await;
    let log: Vec<(String, String)> =
        sqlx::query_as("SELECT entity, op FROM change_log ORDER BY seq")
            .fetch_all(&core.pool())
            .await
            .unwrap();
    assert!(log.contains(&("company".into(), "insert".into())));
    assert!(log.contains(&("users".into(), "insert".into())));

    let err = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&me.user.id)
        .execute(&core.pool())
        .await;
    assert!(err.is_err(), "DELETE físico deveria ser barrado");
}

#[tokio::test]
async fn audit_log_is_append_only() {
    let (_dir, core, _me) = onboarded().await;
    let pool = core.pool();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_logs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(n >= 2);
    assert!(sqlx::query("UPDATE audit_logs SET action = 'x'")
        .execute(&pool)
        .await
        .is_err());
    assert!(sqlx::query("DELETE FROM audit_logs")
        .execute(&pool)
        .await
        .is_err());
}

#[tokio::test]
async fn installation_identity_is_stable_across_reopen() {
    let (dir, core) = setup().await;
    let first = core.installation();
    drop(core);
    let again = nucleo_core::Core::open(dir.path()).await.unwrap();
    assert_eq!(first.device_id, again.installation().device_id);
    assert_eq!(first.branch_id, again.installation().branch_id);
}

#[test]
fn ids_are_time_ordered_uuid_v7() {
    let a = db::new_id();
    let b = db::new_id();
    assert_eq!(a.len(), 36);
    assert_eq!(&a[14..15], "7");
    assert!(a < b);
}
