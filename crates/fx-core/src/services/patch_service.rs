//! Entity patch service: patch-based edit history for structured entities.
//!
//! Each edit is recorded as a JSON Patch (RFC 6902) — an array of operations
//! like `[{"op":"replace","path":"/title","value":"New"}]`.
//!
//! The entity table (courses, books) stores materialized current state.
//! Patches are applied atomically: update entity + insert patch in one transaction.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

// ── Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PatchRow {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub author_did: String,
    pub operations: Value,
    pub summary: String,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchOp {
    pub op: String,          // "add", "remove", "replace"
    pub path: String,        // JSON Pointer, e.g. "/title", "/schedule/3/topic"
    #[serde(default)]
    pub value: Option<Value>,
}

// ── Apply JSON Patch to a JSONB document ────────────────────────────────

/// Apply RFC 6902 JSON Patch operations to a JSON value.
pub fn apply_patch(doc: &mut Value, ops: &[PatchOp]) -> Result<(), String> {
    for op in ops {
        match op.op.as_str() {
            "replace" => {
                let val = op.value.as_ref().ok_or("replace requires value")?;
                set_path(doc, &op.path, val.clone())?;
            }
            "add" => {
                let val = op.value.as_ref().ok_or("add requires value")?;
                add_path(doc, &op.path, val.clone())?;
            }
            "remove" => {
                remove_path(doc, &op.path)?;
            }
            other => return Err(format!("unsupported op: {other}")),
        }
    }
    Ok(())
}

fn set_path(doc: &mut Value, path: &str, value: Value) -> Result<(), String> {
    if path == "" || path == "/" {
        *doc = value;
        return Ok(());
    }
    let parts = parse_pointer(path);
    let parent = navigate_mut(doc, &parts[..parts.len() - 1])?;
    let last = &parts[parts.len() - 1];

    match parent {
        Value::Object(map) => { map.insert(last.to_string(), value); Ok(()) }
        Value::Array(arr) => {
            let idx: usize = last.parse().map_err(|_| format!("invalid array index: {last}"))?;
            if idx < arr.len() { arr[idx] = value; Ok(()) }
            else { Err(format!("array index out of bounds: {idx}")) }
        }
        _ => Err(format!("cannot set on non-container at {path}")),
    }
}

fn add_path(doc: &mut Value, path: &str, value: Value) -> Result<(), String> {
    let parts = parse_pointer(path);
    let parent = navigate_mut(doc, &parts[..parts.len() - 1])?;
    let last = &parts[parts.len() - 1];

    match parent {
        Value::Object(map) => { map.insert(last.to_string(), value); Ok(()) }
        Value::Array(arr) => {
            if last == "-" {
                arr.push(value);
            } else {
                let idx: usize = last.parse().map_err(|_| format!("invalid array index: {last}"))?;
                if idx <= arr.len() { arr.insert(idx, value); }
                else { arr.push(value); }
            }
            Ok(())
        }
        _ => Err(format!("cannot add to non-container at {path}")),
    }
}

fn remove_path(doc: &mut Value, path: &str) -> Result<(), String> {
    let parts = parse_pointer(path);
    let parent = navigate_mut(doc, &parts[..parts.len() - 1])?;
    let last = &parts[parts.len() - 1];

    match parent {
        Value::Object(map) => { map.remove(last.as_str()); Ok(()) }
        Value::Array(arr) => {
            let idx: usize = last.parse().map_err(|_| format!("invalid array index: {last}"))?;
            if idx < arr.len() { arr.remove(idx); Ok(()) }
            else { Err(format!("array index out of bounds: {idx}")) }
        }
        _ => Err(format!("cannot remove from non-container at {path}")),
    }
}

fn parse_pointer(path: &str) -> Vec<String> {
    path.trim_start_matches('/')
        .split('/')
        .map(|s| s.replace("~1", "/").replace("~0", "~"))
        .collect()
}

fn navigate_mut<'a>(doc: &'a mut Value, parts: &[String]) -> Result<&'a mut Value, String> {
    let mut current = doc;
    for part in parts {
        current = match current {
            Value::Object(map) => map.get_mut(part.as_str())
                .ok_or_else(|| format!("path not found: {part}"))?,
            Value::Array(arr) => {
                let idx: usize = part.parse().map_err(|_| format!("invalid index: {part}"))?;
                arr.get_mut(idx).ok_or_else(|| format!("index out of bounds: {idx}"))?
            }
            _ => return Err(format!("cannot navigate into non-container at {part}")),
        };
    }
    Ok(current)
}

// ── Diff: compute patch between two JSON values ─────────────────────────

/// Compute the JSON Patch needed to transform `old` into `new`.
pub fn diff(old: &Value, new: &Value) -> Vec<PatchOp> {
    let mut ops = Vec::new();
    diff_recursive("", old, new, &mut ops);
    ops
}

fn diff_recursive(path: &str, old: &Value, new: &Value, ops: &mut Vec<PatchOp>) {
    if old == new { return; }

    match (old, new) {
        (Value::Object(old_map), Value::Object(new_map)) => {
            // Removed keys
            for key in old_map.keys() {
                if !new_map.contains_key(key) {
                    ops.push(PatchOp { op: "remove".into(), path: format!("{path}/{key}"), value: None });
                }
            }
            // Added or changed keys
            for (key, new_val) in new_map {
                let child_path = format!("{path}/{key}");
                match old_map.get(key) {
                    Some(old_val) => diff_recursive(&child_path, old_val, new_val, ops),
                    None => ops.push(PatchOp { op: "add".into(), path: child_path, value: Some(new_val.clone()) }),
                }
            }
        }
        _ => {
            // Primitive or structural type change: replace
            ops.push(PatchOp { op: "replace".into(), path: path.to_string(), value: Some(new.clone()) });
        }
    }
}

// ── Database operations ─────────────────────────────────────────────────

/// Record a patch. If the author is the entity owner, auto-apply; otherwise pending.
pub async fn create_patch(
    pool: &PgPool,
    entity_type: &str,
    entity_id: &str,
    author_did: &str,
    owner_did: &str,
    operations: &[PatchOp],
    summary: &str,
) -> crate::Result<PatchRow> {
    let id = format!("p-{}", crate::util::tid());
    let ops_json = serde_json::to_value(operations)?;
    let status = if author_did == owner_did { "applied" } else { "pending" };

    sqlx::query(
        "INSERT INTO entity_patches (id, entity_type, entity_id, author_did, operations, summary, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id).bind(entity_type).bind(entity_id)
    .bind(author_did).bind(&ops_json).bind(summary).bind(status)
    .execute(pool).await?;

    get_patch(pool, &id).await
}

pub async fn get_patch(pool: &PgPool, id: &str) -> crate::Result<PatchRow> {
    sqlx::query_as::<_, PatchRow>(
        "SELECT id, entity_type, entity_id, author_did, operations, summary, status, \
         reviewed_by, reviewed_at, created_at FROM entity_patches WHERE id = $1",
    ).bind(id).fetch_one(pool).await
    .map_err(|_| crate::Error::NotFound { entity: "patch", id: id.to_string() })
}

/// List patches for an entity, newest first.
pub async fn list_patches(
    pool: &PgPool,
    entity_type: &str,
    entity_id: &str,
) -> crate::Result<Vec<PatchRow>> {
    Ok(sqlx::query_as::<_, PatchRow>(
        "SELECT id, entity_type, entity_id, author_did, operations, summary, status, \
         reviewed_by, reviewed_at, created_at \
         FROM entity_patches WHERE entity_type = $1 AND entity_id = $2 \
         ORDER BY created_at DESC",
    ).bind(entity_type).bind(entity_id).fetch_all(pool).await?)
}

/// List pending patches for review.
pub async fn list_pending(
    pool: &PgPool,
    entity_type: &str,
    entity_id: &str,
) -> crate::Result<Vec<PatchRow>> {
    Ok(sqlx::query_as::<_, PatchRow>(
        "SELECT id, entity_type, entity_id, author_did, operations, summary, status, \
         reviewed_by, reviewed_at, created_at \
         FROM entity_patches WHERE entity_type = $1 AND entity_id = $2 AND status = 'pending' \
         ORDER BY created_at",
    ).bind(entity_type).bind(entity_id).fetch_all(pool).await?)
}

/// Approve a pending patch (reviewer applies it).
pub async fn approve_patch(pool: &PgPool, patch_id: &str, reviewer_did: &str) -> crate::Result<PatchRow> {
    sqlx::query(
        "UPDATE entity_patches SET status = 'applied', reviewed_by = $1, reviewed_at = NOW() \
         WHERE id = $2 AND status = 'pending'",
    ).bind(reviewer_did).bind(patch_id).execute(pool).await?;
    get_patch(pool, patch_id).await
}

/// Reject a pending patch.
pub async fn reject_patch(pool: &PgPool, patch_id: &str, reviewer_did: &str) -> crate::Result<PatchRow> {
    sqlx::query(
        "UPDATE entity_patches SET status = 'rejected', reviewed_by = $1, reviewed_at = NOW() \
         WHERE id = $2 AND status = 'pending'",
    ).bind(reviewer_did).bind(patch_id).execute(pool).await?;
    get_patch(pool, patch_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_apply_replace() {
        let mut doc = json!({"title": "Old", "code": "CS100"});
        let ops = vec![PatchOp { op: "replace".into(), path: "/title".into(), value: Some(json!("New")) }];
        apply_patch(&mut doc, &ops).unwrap();
        assert_eq!(doc["title"], "New");
        assert_eq!(doc["code"], "CS100");
    }

    #[test]
    fn test_apply_add_to_array() {
        let mut doc = json!({"tags": ["a", "b"]});
        let ops = vec![PatchOp { op: "add".into(), path: "/tags/-".into(), value: Some(json!("c")) }];
        apply_patch(&mut doc, &ops).unwrap();
        assert_eq!(doc["tags"], json!(["a", "b", "c"]));
    }

    #[test]
    fn test_apply_remove() {
        let mut doc = json!({"title": "X", "extra": "Y"});
        let ops = vec![PatchOp { op: "remove".into(), path: "/extra".into(), value: None }];
        apply_patch(&mut doc, &ops).unwrap();
        assert!(doc.get("extra").is_none());
    }

    #[test]
    fn test_apply_nested() {
        let mut doc = json!({"schedule": [{"session": 1, "topic": "Old"}]});
        let ops = vec![PatchOp { op: "replace".into(), path: "/schedule/0/topic".into(), value: Some(json!("New")) }];
        apply_patch(&mut doc, &ops).unwrap();
        assert_eq!(doc["schedule"][0]["topic"], "New");
    }

    #[test]
    fn test_diff_simple() {
        let old = json!({"title": "A", "code": "CS1"});
        let new = json!({"title": "B", "code": "CS1"});
        let ops = diff(&old, &new);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].path, "/title");
        assert_eq!(ops[0].op, "replace");
    }

    #[test]
    fn test_diff_add_remove() {
        let old = json!({"a": 1, "b": 2});
        let new = json!({"a": 1, "c": 3});
        let ops = diff(&old, &new);
        assert!(ops.iter().any(|o| o.op == "remove" && o.path == "/b"));
        assert!(ops.iter().any(|o| o.op == "add" && o.path == "/c"));
    }

    #[test]
    fn test_roundtrip() {
        let old = json!({"title": "A", "tags": ["x"], "nested": {"a": 1}});
        let new = json!({"title": "B", "tags": ["x", "y"], "nested": {"a": 2}});
        let ops = diff(&old, &new);
        let mut doc = old.clone();
        apply_patch(&mut doc, &ops).unwrap();
        assert_eq!(doc, new);
    }
}
