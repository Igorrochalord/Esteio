//! Registro de auditoria (append-only: triggers no SQLite barram UPDATE/DELETE).
//! Snapshots vêm do `Serialize` do modelo, que já omite campos sensíveis
//! (`password_hash`, bytes do logo) com `#[serde(skip_serializing)]`.

use serde::Serialize;
use serde_json::{Map, Value};

use crate::db::{self, Conn, Installation};
use crate::error::CoreResult;

pub fn snapshot<T: Serialize>(obj: &T) -> Value {
    serde_json::to_value(obj).unwrap_or(Value::Null)
}

/// Reduz before/after aos campos que mudaram (ignora `updated_at`).
pub fn diff(before: &Value, after: &Value) -> (Value, Value) {
    let (Some(b), Some(a)) = (before.as_object(), after.as_object()) else {
        return (before.clone(), after.clone());
    };
    let mut ob = Map::new();
    let mut oa = Map::new();
    for (k, v) in a {
        if k == "updated_at" || b.get(k) == Some(v) {
            continue;
        }
        ob.insert(k.clone(), b.get(k).cloned().unwrap_or(Value::Null));
        oa.insert(k.clone(), v.clone());
    }
    (Value::Object(ob), Value::Object(oa))
}

pub fn is_empty(v: &Value) -> bool {
    v.as_object().is_some_and(Map::is_empty)
}

pub struct Entry<'a> {
    pub actor_id: Option<&'a str>,
    pub action: &'a str,
    pub entity: &'a str,
    pub entity_id: Option<&'a str>,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

pub async fn record(conn: &mut Conn, inst: &Installation, e: Entry<'_>) -> CoreResult<()> {
    sqlx::query(
        "INSERT INTO audit_logs
           (id, user_id, action, entity, entity_id, before, after, branch_id, device_id, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(db::new_id())
    .bind(e.actor_id)
    .bind(e.action)
    .bind(e.entity)
    .bind(e.entity_id)
    .bind(e.before.map(|v| v.to_string()))
    .bind(e.after.map(|v| v.to_string()))
    .bind(&inst.branch_id)
    .bind(&inst.device_id)
    .bind(db::now())
    .execute(conn)
    .await?;
    Ok(())
}

/// Recebe as fotos completas em `before`/`after`, grava só os campos alterados
/// e não grava nada se nada mudou.
pub async fn record_changes(conn: &mut Conn, inst: &Installation, e: Entry<'_>) -> CoreResult<()> {
    let (b, a) = diff(
        e.before.as_ref().unwrap_or(&Value::Null),
        e.after.as_ref().unwrap_or(&Value::Null),
    );
    if is_empty(&a) {
        return Ok(());
    }
    record(
        conn,
        inst,
        Entry {
            before: Some(b),
            after: Some(a),
            ..e
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn diff_keeps_only_changed_fields() {
        let (b, a) = diff(
            &json!({"name": "A", "role": "user", "updated_at": "1"}),
            &json!({"name": "B", "role": "user", "updated_at": "2"}),
        );
        assert_eq!(b, json!({"name": "A"}));
        assert_eq!(a, json!({"name": "B"}));
    }
}
