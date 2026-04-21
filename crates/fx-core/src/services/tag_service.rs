//! Tag / name service.
//!
//! Two entities:
//!
//! * **Tag** (concept) — `tags.id`, opaque `tg-xxxxxxxxxxxxxxxx`. Every
//!   edge table in the system (content_teaches, content_prereqs, skill
//!   tree edges, …) references this id. The concept itself has no
//!   display string.
//!
//! * **Name** — `tag_names.id`, opaque `tn-xxxxxxxxxxxxxxxx`. One row
//!   per `(tag_id, lang, string)`. A concept has 1..N names; none of
//!   them is "primary" — which to show is a viewer decision:
//!
//!     1. `user_name_pref(did, tag_id)` if the viewer has one.
//!     2. Earliest-added name in the viewer's UI locale.
//!     3. Earliest-added name in `en`.
//!     4. Earliest-added name in any language.
//!
//! The `Tag` DTO represents a single name row; its `names` field is
//! computed from `tag_label_map(tag_id)` at query time for frontend
//! display convenience.

use std::collections::HashMap;

use sqlx::PgPool;

use crate::models::{CreateTag, Tag};
use crate::Result;

const TAG_SELECT: &str = "SELECT n.id, n.name, tag_label_map(n.tag_id) AS names, \
                                 n.added_at, n.tag_id, n.lang \
                          FROM tag_names n";

async fn audit(
    conn: &mut sqlx::PgConnection,
    action: &str,
    actor_did: &str,
    tag_id: Option<&str>,
    name: Option<&str>,
    lang: Option<&str>,
    merged_into: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO tag_audit_log (action, actor_did, tag_id, name, lang, merged_into) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(action).bind(actor_did).bind(tag_id).bind(name).bind(lang).bind(merged_into)
    .execute(&mut *conn).await?;
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════
// Reads
// ══════════════════════════════════════════════════════════════════════

/// List every name row (used by the admin tag browser and the
/// frontend's tagStore startup load).
pub async fn list_tags(pool: &PgPool, limit: i64) -> Result<Vec<Tag>> {
    let rows = sqlx::query_as::<_, Tag>(
        &format!("{TAG_SELECT} ORDER BY n.name LIMIT $1"),
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get one record for a tag reference. Accepts either a name id
/// (`tn-…`) or a tag id (`tg-…`). For a tag id, picks the
/// earliest-added name in the preferred locale → en → any.
pub async fn get_tag(pool: &PgPool, id: &str) -> Result<Tag> {
    get_tag_with_lang(pool, id, "en").await
}

pub async fn get_tag_with_lang(pool: &PgPool, id: &str, preferred_lang: &str) -> Result<Tag> {
    // By name id.
    if let Some(tag) = sqlx::query_as::<_, Tag>(
        &format!("{TAG_SELECT} WHERE n.id = $1"),
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    {
        return Ok(tag);
    }
    // By concept tag_id → pick earliest-added in locale preference.
    if let Some(tag) = sqlx::query_as::<_, Tag>(
        &format!("{TAG_SELECT} WHERE n.tag_id = $1 \
                  ORDER BY (n.lang = $2) DESC, (n.lang = 'en') DESC, n.added_at ASC, n.id \
                  LIMIT 1"),
    )
    .bind(id)
    .bind(preferred_lang)
    .fetch_optional(pool)
    .await?
    {
        return Ok(tag);
    }
    Err(crate::Error::NotFound {
        entity: "tag",
        id: id.to_string(),
    })
}

/// Batch-fetch a display name for each tag_id in `tag_ids`. Result map
/// always contains every input id (falls back to the id itself if the
/// tag has no names registered).
pub async fn get_tag_names(
    pool: &PgPool,
    tag_ids: &[String],
) -> Result<HashMap<String, String>> {
    if tag_ids.is_empty() {
        return Ok(HashMap::new());
    }
    #[derive(sqlx::FromRow)]
    struct Row {
        tag_id: String,
        name: String,
    }
    let rows = sqlx::query_as::<_, Row>(
        "SELECT DISTINCT ON (tag_id) tag_id, name \
         FROM tag_names \
         WHERE tag_id = ANY($1) \
         ORDER BY tag_id, (lang = 'en') DESC, added_at ASC, id",
    )
    .bind(tag_ids)
    .fetch_all(pool)
    .await?;
    let mut map: HashMap<String, String> = rows.into_iter().map(|r| (r.tag_id, r.name)).collect();
    for t in tag_ids {
        map.entry(t.clone()).or_insert_with(|| t.clone());
    }
    Ok(map)
}

/// Batch-fetch `{lang → earliest-added name}` for each tag_id.
pub async fn get_tag_names_i18n(
    pool: &PgPool,
    tag_ids: &[String],
) -> Result<HashMap<String, HashMap<String, String>>> {
    if tag_ids.is_empty() {
        return Ok(HashMap::new());
    }
    #[derive(sqlx::FromRow)]
    struct Row {
        tag_id: String,
        names: sqlx::types::Json<HashMap<String, String>>,
    }
    let rows = sqlx::query_as::<_, Row>(
        "SELECT DISTINCT tag_id, tag_label_map(tag_id) AS names \
         FROM tag_names WHERE tag_id = ANY($1)",
    )
    .bind(tag_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| (r.tag_id, r.names.0)).collect())
}

/// Substring search across every name. A `zh` user typing "线性" sees
/// "线性代数" / "线性逻辑"; an `en` user typing "linear" sees
/// "Linear Algebra" / "Linear Logic". Each match is its own row; the
/// caller groups by `tag_id` if they want concepts.
pub async fn search_tags(pool: &PgPool, query: &str, limit: i64) -> Result<Vec<Tag>> {
    let pattern = format!("%{query}%");
    let rows = sqlx::query_as::<_, Tag>(
        &format!("{TAG_SELECT} \
                  WHERE n.name ILIKE $1 \
                  ORDER BY \
                      CASE WHEN n.name = $2 THEN 0 \
                           WHEN n.name ILIKE $3 THEN 1 \
                           ELSE 2 END, \
                      n.name \
                  LIMIT $4"),
    )
    .bind(&pattern)
    .bind(query)
    .bind(format!("{query}%"))
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// All names attached to a tag, ordered English-first then by lang,
/// earliest-added within a language. Used by the TagDetail page.
pub async fn list_names_for_tag(pool: &PgPool, tag_id: &str) -> Result<Vec<Tag>> {
    let rows = sqlx::query_as::<_, Tag>(
        &format!("{TAG_SELECT} \
                  WHERE n.tag_id = $1 \
                  ORDER BY (n.lang = 'en') DESC, n.lang, n.added_at, n.id"),
    )
    .bind(tag_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List every name id in the same concept as `reference`. `reference`
/// can be a name id or tag id. Used by queries that want to expand
/// "this label" to "all labels on this concept".
pub async fn list_group_members(pool: &PgPool, reference: &str) -> Result<Vec<String>> {
    let tag_id = resolve_reference_to_tag_id(pool, reference).await?;
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM tag_names WHERE tag_id = $1 \
         ORDER BY (lang = 'en') DESC, lang, added_at, id",
    )
    .bind(&tag_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Sibling-listing variant that returns full Tag rows.
pub async fn list_group_siblings(pool: &PgPool, reference: &str) -> Result<Vec<Tag>> {
    let tag_id = resolve_reference_to_tag_id(pool, reference).await?;
    list_names_for_tag(pool, &tag_id).await
}

/// Dedupe a list of name ids so that at most one name per concept
/// appears, preserving input order.
pub async fn dedupe_by_group(pool: &PgPool, name_ids: &[String]) -> Result<Vec<String>> {
    if name_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, tag_id FROM tag_names WHERE id = ANY($1)",
    )
    .bind(name_ids)
    .fetch_all(pool)
    .await?;
    let mut tag_of: HashMap<String, String> = HashMap::new();
    for (id, t) in &rows { tag_of.insert(id.clone(), t.clone()); }
    let mut seen: HashMap<String, ()> = HashMap::new();
    let mut out = Vec::new();
    for id in name_ids {
        if let Some(t) = tag_of.get(id) {
            if !seen.contains_key(t) { seen.insert(t.clone(), ()); out.push(id.clone()); }
        } else {
            out.push(id.clone());
        }
    }
    Ok(out)
}

/// Expand a list of name ids to include every sibling name in the
/// same concept. Used by queries that take a label and need to match
/// against all its translations.
pub async fn expand_to_group(pool: &PgPool, name_ids: &[String]) -> Result<Vec<String>> {
    if name_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT id FROM tag_names \
         WHERE tag_id IN (SELECT tag_id FROM tag_names WHERE id = ANY($1))",
    )
    .bind(name_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

async fn resolve_reference_to_tag_id(pool: &PgPool, reference: &str) -> Result<String> {
    if reference.starts_with("tg-") {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tags WHERE id = $1)")
            .bind(reference).fetch_one(pool).await?;
        if exists { return Ok(reference.to_string()); }
    }
    let tag_id: Option<String> = sqlx::query_scalar(
        "SELECT tag_id FROM tag_names WHERE id = $1",
    )
    .bind(reference).fetch_optional(pool).await?;
    tag_id.ok_or_else(|| crate::Error::NotFound {
        entity: "tag",
        id: reference.to_string(),
    })
}

// ══════════════════════════════════════════════════════════════════════
// Input boundary
// ══════════════════════════════════════════════════════════════════════

/// Format check: is `input` a `tg-…` id? Cheap; no DB round trip.
/// Used at user-facing write boundaries that expect the client to have
/// already resolved user input via `POST /api/tags/resolve`.
pub fn require_tag_id(input: &str) -> Result<()> {
    if input.starts_with("tg-") {
        Ok(())
    } else {
        Err(crate::Error::BadRequest(format!(
            "expected a tag_id (tg-…), got {input:?} — resolve labels via POST /api/tags/resolve first"
        )))
    }
}

/// Read-only resolve: map a user-supplied string to its `tag_id`, or
/// None. Used by query endpoints (`?tag_id=Math`) where a typo should
/// yield empty results, not a new tag.
pub async fn lookup_tag_id(pool: &PgPool, input: &str) -> Result<Option<String>> {
    if input.starts_with("tg-") {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tags WHERE id = $1)")
            .bind(input).fetch_one(pool).await?;
        if exists { return Ok(Some(input.to_string())); }
    }
    // By name id.
    if let Some(t) = sqlx::query_scalar::<_, String>(
        "SELECT tag_id FROM tag_names WHERE id = $1",
    )
    .bind(input).fetch_optional(pool).await? {
        return Ok(Some(t));
    }
    // By name string — if unique across languages, accept.
    let matches: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT tag_id FROM tag_names WHERE name = $1",
    )
    .bind(input).fetch_all(pool).await?;
    if matches.len() == 1 {
        return Ok(Some(matches.into_iter().next().unwrap().0));
    }
    Ok(None)
}

/// Input boundary: user typed / picked a label string. Returns the
/// canonical `tag_id`, creating a fresh concept + name row if the
/// string is brand new. Rejects `tg-…` input — if the caller already
/// has a tag_id they don't need this.
pub async fn resolve_tag_id(
    conn: &mut sqlx::PgConnection,
    input: &str,
    actor_did: &str,
) -> Result<String> {
    if input.starts_with("tg-") {
        return Err(crate::Error::BadRequest(format!(
            "resolve_tag_id expects a label or new name, got a tag_id: {input}"
        )));
    }
    // By name id.
    if let Some(t) = sqlx::query_scalar::<_, String>(
        "SELECT tag_id FROM tag_names WHERE id = $1",
    )
    .bind(input).fetch_optional(&mut *conn).await? {
        return Ok(t);
    }
    // By name string (any language).
    let matches: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT tag_id FROM tag_names WHERE name = $1",
    )
    .bind(input).fetch_all(&mut *conn).await?;
    if matches.len() == 1 {
        return Ok(matches.into_iter().next().unwrap().0);
    }
    if matches.len() > 1 {
        return Err(crate::Error::BadRequest(format!(
            "{input:?} is ambiguous — it names {} different concepts; pick by tag_id", matches.len()
        )));
    }
    // Brand-new: mint a tag + first name.
    let (tag_id, _name_id) = create_tag_with_name(conn, input, guess_lang(input), actor_did).await?;
    Ok(tag_id)
}

// ══════════════════════════════════════════════════════════════════════
// Lifecycle
// ══════════════════════════════════════════════════════════════════════

/// Create a new concept with an initial name plus any extra
/// per-language names in `input.names`.
pub async fn create_tag(pool: &PgPool, input: &CreateTag, actor_did: &str) -> Result<Tag> {
    let initial_name = input.name.trim();
    if initial_name.is_empty() {
        return Err(crate::Error::BadRequest("name required".into()));
    }
    let initial_lang = guess_lang(initial_name);
    let mut conn = pool.acquire().await?;
    let (tag_id, first_name_id) =
        create_tag_with_name(&mut conn, initial_name, initial_lang, actor_did).await?;
    if let Some(extras) = &input.names {
        for (lang, name) in extras.iter() {
            let name = name.trim();
            if name.is_empty() || lang == initial_lang { continue; }
            let _ = add_name_conn(&mut conn, &tag_id, name, lang, actor_did).await;
        }
    }
    get_tag(pool, &first_name_id).await
}

/// Add another name for an existing concept. Returns the new Tag row.
/// No-op (no new row, returns existing) if the same (tag_id, name,
/// lang) already exists.
pub async fn add_name(
    pool: &PgPool,
    tag_id: &str,
    name: &str,
    lang: &str,
    actor_did: &str,
) -> Result<Tag> {
    let name = name.trim();
    if name.is_empty() {
        return Err(crate::Error::BadRequest("name required".into()));
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tags WHERE id = $1)")
        .bind(tag_id).fetch_one(pool).await?;
    if !exists {
        return Err(crate::Error::NotFound { entity: "tag", id: tag_id.to_string() });
    }
    let mut conn = pool.acquire().await?;
    let name_id = add_name_conn(&mut conn, tag_id, name, lang, actor_did).await?;
    get_tag(pool, &name_id).await
}

async fn add_name_conn(
    conn: &mut sqlx::PgConnection,
    tag_id: &str,
    name: &str,
    lang: &str,
    actor_did: &str,
) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM tag_names WHERE tag_id = $1 AND name = $2 AND lang = $3",
    )
    .bind(tag_id).bind(name).bind(lang)
    .fetch_optional(&mut *conn).await?;
    if let Some(id) = existing { return Ok(id); }

    let name_id: String = sqlx::query_scalar(
        "INSERT INTO tag_names (tag_id, name, lang) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(tag_id).bind(name).bind(lang)
    .fetch_one(&mut *conn).await?;
    audit(conn, "add_name", actor_did, Some(tag_id), Some(name), Some(lang), None).await?;
    Ok(name_id)
}

async fn create_tag_with_name(
    conn: &mut sqlx::PgConnection,
    name: &str,
    lang: &str,
    actor_did: &str,
) -> Result<(String, String)> {
    let tag_id: String = sqlx::query_scalar("INSERT INTO tags DEFAULT VALUES RETURNING id")
        .fetch_one(&mut *conn).await?;
    let name_id: String = sqlx::query_scalar(
        "INSERT INTO tag_names (tag_id, name, lang) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&tag_id).bind(name).bind(lang)
    .fetch_one(&mut *conn).await?;
    audit(conn, "create_tag", actor_did, Some(&tag_id), Some(name), Some(lang), None).await?;
    Ok((tag_id, name_id))
}

/// Remove a single name. If its concept has no names left, drop the
/// concept (and cascade every edge that referenced it).
pub async fn remove_name(pool: &PgPool, name_id: &str, actor_did: &str) -> Result<()> {
    #[derive(sqlx::FromRow)]
    struct Row { tag_id: String, name: String, lang: String }
    let row: Option<Row> = sqlx::query_as(
        "SELECT tag_id, name, lang FROM tag_names WHERE id = $1",
    )
    .bind(name_id).fetch_optional(pool).await?;
    let Some(row) = row else { return Ok(()); };
    let mut conn = pool.acquire().await?;
    sqlx::query("DELETE FROM tag_names WHERE id = $1")
        .bind(name_id).execute(&mut *conn).await?;
    audit(&mut conn, "remove_name", actor_did, Some(&row.tag_id), Some(&row.name), Some(&row.lang), None).await?;
    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tag_names WHERE tag_id = $1",
    )
    .bind(&row.tag_id).fetch_one(&mut *conn).await?;
    if remaining == 0 {
        sqlx::query("DELETE FROM tags WHERE id = $1")
            .bind(&row.tag_id).execute(&mut *conn).await?;
    }
    Ok(())
}

/// Merge two concepts: every name of `from_tag_id` moves to
/// `into_tag_id`, then `from_tag_id` is deleted. Edges that were
/// pointing at `from_tag_id` cascade-delete — callers who care about
/// preserving them must migrate them before merging.
pub async fn merge_tags(
    pool: &PgPool,
    into_tag_id: &str,
    from_tag_id: &str,
    actor_did: &str,
) -> Result<()> {
    if into_tag_id == from_tag_id { return Ok(()); }
    let mut conn = pool.acquire().await?;
    sqlx::query("UPDATE tag_names SET tag_id = $1 WHERE tag_id = $2")
        .bind(into_tag_id).bind(from_tag_id).execute(&mut *conn).await?;
    sqlx::query("UPDATE user_name_pref SET tag_id = $1 WHERE tag_id = $2")
        .bind(into_tag_id).bind(from_tag_id).execute(&mut *conn).await?;
    sqlx::query("DELETE FROM tags WHERE id = $1")
        .bind(from_tag_id).execute(&mut *conn).await?;
    audit(&mut conn, "merge_tag", actor_did, Some(from_tag_id), None, None, Some(into_tag_id)).await?;
    Ok(())
}

/// Accepts name ids or tag ids on both sides.
pub async fn merge_tag(pool: &PgPool, from: &str, into: &str, actor_did: &str) -> Result<()> {
    let from_tag = resolve_reference_to_tag_id(pool, from).await?;
    let into_tag = resolve_reference_to_tag_id(pool, into).await?;
    merge_tags(pool, &into_tag, &from_tag, actor_did).await
}

/// Bulk-add per-language names for a concept. Input is `{lang → name}`;
/// each entry becomes a `tag_names` row (idempotent — a matching row
/// is left alone). Existing names are never removed by this call; to
/// remove, use `remove_name`.
pub async fn update_tag_names(
    pool: &PgPool,
    reference: &str,
    names: &HashMap<String, String>,
    actor_did: &str,
) -> Result<Tag> {
    let tag_id = resolve_reference_to_tag_id(pool, reference).await?;
    let mut conn = pool.acquire().await?;
    let mut latest_name_id: Option<String> = None;
    for (lang, name) in names.iter() {
        let name = name.trim();
        if name.is_empty() { continue; }
        let id = add_name_conn(&mut conn, &tag_id, name, lang, actor_did).await?;
        latest_name_id = Some(id);
    }
    let show = latest_name_id.unwrap_or(tag_id.clone());
    get_tag(pool, &show).await
}

// ══════════════════════════════════════════════════════════════════════
// User preference
// ══════════════════════════════════════════════════════════════════════

/// Set the viewer's preferred name for a concept. The name must belong
/// to the same concept; otherwise error.
pub async fn set_user_name_pref(
    pool: &PgPool,
    did: &str,
    tag_id: &str,
    name_id: &str,
) -> Result<()> {
    let ok: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tag_names WHERE id = $1 AND tag_id = $2)",
    )
    .bind(name_id).bind(tag_id).fetch_one(pool).await?;
    if !ok {
        return Err(crate::Error::Validation(vec![crate::validation::ValidationError {
            field: "name_id".into(),
            message: "name does not belong to this tag".into(),
        }]));
    }
    sqlx::query(
        "INSERT INTO user_name_pref (did, tag_id, name_id) VALUES ($1, $2, $3) \
         ON CONFLICT (did, tag_id) DO UPDATE SET name_id = EXCLUDED.name_id, chosen_at = NOW()",
    )
    .bind(did).bind(tag_id).bind(name_id)
    .execute(pool).await?;
    Ok(())
}

pub async fn clear_user_name_pref(pool: &PgPool, did: &str, tag_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM user_name_pref WHERE did = $1 AND tag_id = $2")
        .bind(did).bind(tag_id).execute(pool).await?;
    Ok(())
}

/// Return a `{tag_id → name_id}` map of this user's preferences.
pub async fn list_user_name_prefs(
    pool: &PgPool,
    did: &str,
) -> Result<HashMap<String, String>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT tag_id, name_id FROM user_name_pref WHERE did = $1",
    )
    .bind(did).fetch_all(pool).await?;
    Ok(rows.into_iter().collect())
}

// ══════════════════════════════════════════════════════════════════════
// Taxonomy derivation — unchanged (operates on tag_id only)
// ══════════════════════════════════════════════════════════════════════

/// Derive the set of "topic" tags for a content — the transitive
/// closure of the content's teach tags' ancestors in `tag_parents`.
pub async fn derive_topics(pool: &PgPool, content_uri: &str) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        WITH RECURSIVE teach AS (
            SELECT tag_id FROM content_teaches WHERE content_uri = $1
        ),
        ancestors AS (
            SELECT tp.parent_tag AS tag_id, 1 AS depth
            FROM tag_parents tp WHERE tp.child_tag IN (SELECT tag_id FROM teach)
            UNION
            SELECT tp.parent_tag, a.depth + 1
            FROM tag_parents tp JOIN ancestors a ON tp.child_tag = a.tag_id
            WHERE a.depth < 10
        )
        SELECT DISTINCT tag_id FROM ancestors
        WHERE tag_id NOT IN (SELECT tag_id FROM teach)
        ORDER BY tag_id
        "#,
    )
    .bind(content_uri).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

// ══════════════════════════════════════════════════════════════════════
// Audit log
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct TagAuditEntry {
    pub id: i64,
    pub action: String,
    pub actor_did: String,
    pub actor_handle: Option<String>,
    pub actor_display_name: Option<String>,
    pub tag_id: Option<String>,
    pub name: Option<String>,
    pub lang: Option<String>,
    pub merged_into: Option<String>,
    pub at: chrono::DateTime<chrono::Utc>,
}

/// Return every audit entry that touched this tag, newest first. An
/// entry is "about this tag" if its `tag_id` matches OR if this tag
/// was the merge destination (so you can see "X was merged into me").
pub async fn list_tag_audit(pool: &PgPool, tag_id: &str, limit: i64) -> Result<Vec<TagAuditEntry>> {
    let rows = sqlx::query_as::<_, TagAuditEntry>(
        "SELECT a.id, a.action, a.actor_did, \
                p.handle AS actor_handle, p.display_name AS actor_display_name, \
                a.tag_id, a.name, a.lang, a.merged_into, a.at \
         FROM tag_audit_log a \
         LEFT JOIN profiles p ON p.did = a.actor_did \
         WHERE a.tag_id = $1 OR a.merged_into = $1 \
         ORDER BY a.at DESC \
         LIMIT $2",
    )
    .bind(tag_id).bind(limit)
    .fetch_all(pool).await?;
    Ok(rows)
}

// ══════════════════════════════════════════════════════════════════════
// Deletion requests (concept-level)
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct TagDeletionRequest {
    pub id: String,
    pub tag_id: String,
    pub requester_did: String,
    pub reason: String,
    pub status: String,
    pub reviewer_did: Option<String>,
    pub review_note: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn request_tag_deletion(
    pool: &PgPool,
    tag_id: &str,
    requester_did: &str,
    reason: &str,
) -> Result<TagDeletionRequest> {
    let existing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tag_deletion_requests \
         WHERE tag_id = $1 AND status = 'pending')",
    )
    .bind(tag_id).fetch_one(pool).await?;
    if existing {
        return Err(crate::Error::Validation(vec![crate::validation::ValidationError {
            field: "tag_id".into(),
            message: "this tag already has a pending deletion request".into(),
        }]));
    }
    let row = sqlx::query_as::<_, TagDeletionRequest>(
        "INSERT INTO tag_deletion_requests (tag_id, requester_did, reason) \
         VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(tag_id).bind(requester_did).bind(reason)
    .fetch_one(pool).await?;
    Ok(row)
}

pub async fn has_pending_deletion(pool: &PgPool, tag_id: &str) -> Result<bool> {
    let existing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tag_deletion_requests \
         WHERE tag_id = $1 AND status = 'pending')",
    )
    .bind(tag_id).fetch_one(pool).await?;
    Ok(existing)
}

pub async fn list_pending_tag_deletions(pool: &PgPool) -> Result<Vec<TagDeletionRequest>> {
    let rows = sqlx::query_as::<_, TagDeletionRequest>(
        "SELECT * FROM tag_deletion_requests WHERE status = 'pending' ORDER BY created_at DESC",
    )
    .fetch_all(pool).await?;
    Ok(rows)
}

pub async fn approve_tag_deletion(
    pool: &PgPool,
    request_id: &str,
    reviewer_did: &str,
    note: Option<&str>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    let tag_id: String = sqlx::query_scalar(
        "UPDATE tag_deletion_requests \
         SET status = 'approved', reviewer_did = $2, review_note = $3, reviewed_at = NOW() \
         WHERE id = $1 AND status = 'pending' RETURNING tag_id",
    )
    .bind(request_id).bind(reviewer_did).bind(note)
    .fetch_optional(&mut *tx).await?
    .ok_or_else(|| crate::Error::NotFound {
        entity: "tag_deletion_request", id: request_id.to_string(),
    })?;
    // Drop the tag; FKs cascade to every edge + name.
    sqlx::query("DELETE FROM tags WHERE id = $1")
        .bind(&tag_id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn reject_tag_deletion(
    pool: &PgPool,
    request_id: &str,
    reviewer_did: &str,
    note: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "UPDATE tag_deletion_requests \
         SET status = 'rejected', reviewer_did = $2, review_note = $3, reviewed_at = NOW() \
         WHERE id = $1 AND status = 'pending'",
    )
    .bind(request_id).bind(reviewer_did).bind(note)
    .execute(pool).await?;
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════════════════

/// Guess an ISO lang code from a string. CJK/hangul/kana → `zh`; else `en`.
/// Good enough for autodetecting the lang of a user-typed new tag name.
fn guess_lang(s: &str) -> &'static str {
    for c in s.chars() {
        let code = c as u32;
        if (0x4E00..=0x9FFF).contains(&code)
            || (0x3400..=0x4DBF).contains(&code)
            || (0x3040..=0x309F).contains(&code)
            || (0x30A0..=0x30FF).contains(&code)
            || (0xAC00..=0xD7AF).contains(&code)
        {
            return "zh";
        }
    }
    "en"
}
