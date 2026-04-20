use std::collections::HashMap;

use sqlx::PgPool;

use crate::models::{CreateTag, Tag};
use crate::Result;

pub async fn list_tags(pool: &PgPool, limit: i64) -> Result<Vec<Tag>> {
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT id, name, names, description, created_by, created_at, group_id, lang FROM tags WHERE removed_at IS NULL ORDER BY name LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(tags)
}

pub async fn get_tag(pool: &PgPool, id: &str) -> Result<Tag> {
    sqlx::query_as::<_, Tag>(
        "SELECT id, name, names, description, created_by, created_at, group_id, lang FROM tags WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::Error::NotFound {
        entity: "tag",
        id: id.to_string(),
    })
}

pub async fn create_tag(pool: &PgPool, input: &CreateTag, created_by: &str) -> Result<Tag> {
    let names_json = serde_json::to_value(
        input.names.as_ref().cloned().unwrap_or_default(),
    )
    .unwrap_or_default();

    sqlx::query(
        "INSERT INTO tags (id, name, names, description, created_by) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&input.id)
    .bind(&input.name)
    .bind(&names_json)
    .bind(&input.description)
    .bind(created_by)
    .execute(pool)
    .await?;

    get_tag(pool, &input.id).await
}

/// Ensure a tag exists (insert if missing, no-op on conflict).
/// Accepts any sqlx Executor (pool or transaction).
/// Detect a tag's default language from its id. CJK characters anywhere
/// → `zh`; otherwise `en`. Used by `ensure_tag` to stamp `lang` on
/// newly-minted tags so group logic treats them correctly.
fn guess_lang_from_id(id: &str) -> &'static str {
    for c in id.chars() {
        let code = c as u32;
        if (0x4E00..=0x9FFF).contains(&code)     // CJK Unified Ideographs
            || (0x3400..=0x4DBF).contains(&code) // CJK Extension A
            || (0x3040..=0x309F).contains(&code) // Hiragana
            || (0x30A0..=0x30FF).contains(&code) // Katakana
            || (0xAC00..=0xD7AF).contains(&code) // Hangul
        {
            return "zh";
        }
    }
    "en"
}

/// Ensure a tag row exists for `tag_id`. If missing, create a fresh
/// alias/translation group containing just this one tag, guess its lang
/// from the id (CJK → `zh`, else `en`), and mark it as the group's rep
/// for that lang. `INSERT ... ON CONFLICT DO NOTHING` still applies, so
/// callers can safely invoke this for tags they suspect may already
/// exist.
pub async fn ensure_tag(
    conn: &mut sqlx::PgConnection,
    tag_id: &str,
    created_by: &str,
) -> Result<()> {
    // Fast path: tag already exists.
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tags WHERE id = $1)")
        .bind(tag_id)
        .fetch_one(&mut *conn)
        .await?;
    if exists {
        return Ok(());
    }
    let lang = guess_lang_from_id(tag_id);
    // Three statements — caller may already be inside a transaction
    // (skill-tree import builds edges in one tx), so we just run
    // sequentially instead of opening a nested one.
    let group_id: String = sqlx::query_scalar(
        "INSERT INTO tag_groups DEFAULT VALUES RETURNING id",
    )
    .fetch_one(&mut *conn)
    .await?;
    let inserted = sqlx::query(
        "INSERT INTO tags (id, name, created_by, lang, group_id, names) \
         VALUES ($1, $2, $3, $4, $5, jsonb_build_object($4::text, $2::text)) \
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(tag_id)
    .bind(tag_id)
    .bind(created_by)
    .bind(lang)
    .bind(&group_id)
    .execute(&mut *conn)
    .await?
    .rows_affected();
    if inserted > 0 {
        sqlx::query(
            "INSERT INTO tag_group_representatives (group_id, lang, tag_id) \
             VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(&group_id)
        .bind(lang)
        .bind(tag_id)
        .execute(&mut *conn)
        .await?;
    } else {
        // Race: some other inserter put the tag in between our check
        // and our INSERT. Clean up the stray group row.
        sqlx::query("DELETE FROM tag_groups WHERE id = $1")
            .bind(&group_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

/// Batch-fetch tag names for a set of IDs. Returns a map of id -> name.
pub async fn get_tag_names(
    pool: &PgPool,
    tag_ids: &[String],
) -> Result<std::collections::HashMap<String, String>> {
    if tag_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    #[derive(sqlx::FromRow)]
    struct TagName {
        id: String,
        name: String,
    }

    let rows = sqlx::query_as::<_, TagName>(
        "SELECT id, name FROM tags WHERE id = ANY($1)",
    )
    .bind(tag_ids)
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<String, String> =
        rows.into_iter().map(|r| (r.id, r.name)).collect();

    // For any IDs not found in DB, use the ID itself as the name
    for id in tag_ids {
        map.entry(id.clone()).or_insert_with(|| id.clone());
    }

    Ok(map)
}

/// Update the i18n names for a tag.
pub async fn update_tag_names(
    pool: &PgPool,
    tag_id: &str,
    names: &HashMap<String, String>,
) -> Result<Tag> {
    let names_json = serde_json::to_value(names).unwrap_or_default();
    sqlx::query("UPDATE tags SET names = $1 WHERE id = $2")
        .bind(&names_json)
        .bind(tag_id)
        .execute(pool)
        .await?;
    get_tag(pool, tag_id).await
}

/// Merge tag `from_id` into `into_id`: migrate all references, then delete the source tag.
pub async fn merge_tag(pool: &PgPool, from_id: &str, into_id: &str) -> Result<()> {
    let mut tx = pool.begin().await?;

    // content_teaches
    sqlx::query(
        "UPDATE content_teaches SET tag_id = $2 WHERE tag_id = $1 \
         AND content_uri NOT IN (SELECT content_uri FROM content_teaches WHERE tag_id = $2)",
    )
    .bind(from_id)
    .bind(into_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM content_teaches WHERE tag_id = $1")
        .bind(from_id)
        .execute(&mut *tx)
        .await?;

    // content_prereqs
    sqlx::query(
        "UPDATE content_prereqs SET tag_id = $2 WHERE tag_id = $1 \
         AND content_uri NOT IN (SELECT content_uri FROM content_prereqs WHERE tag_id = $2)",
    )
    .bind(from_id)
    .bind(into_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM content_prereqs WHERE tag_id = $1")
        .bind(from_id)
        .execute(&mut *tx)
        .await?;

    // content_topics
    sqlx::query(
        "UPDATE content_topics SET tag_id = $2 WHERE tag_id = $1 \
         AND content_uri NOT IN (SELECT content_uri FROM content_topics WHERE tag_id = $2)",
    )
    .bind(from_id)
    .bind(into_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM content_topics WHERE tag_id = $1")
        .bind(from_id)
        .execute(&mut *tx)
        .await?;

    // skill_tree_edges (both parent and child)
    sqlx::query("UPDATE skill_tree_edges SET parent_tag = $2 WHERE parent_tag = $1")
        .bind(from_id)
        .bind(into_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE skill_tree_edges SET child_tag = $2 WHERE child_tag = $1")
        .bind(from_id)
        .bind(into_id)
        .execute(&mut *tx)
        .await?;

    // user_skills
    sqlx::query(
        "UPDATE user_skills SET tag_id = $2 WHERE tag_id = $1 \
         AND did NOT IN (SELECT did FROM user_skills WHERE tag_id = $2)",
    )
    .bind(from_id)
    .bind(into_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM user_skills WHERE tag_id = $1")
        .bind(from_id)
        .execute(&mut *tx)
        .await?;

    // tag_parents (global hierarchy) — rewrite both sides and drop self-loops
    sqlx::query("DELETE FROM tag_parents WHERE parent_tag = $1 AND child_tag = $1")
        .bind(from_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE tag_parents SET parent_tag = $2 WHERE parent_tag = $1 \
         AND NOT EXISTS (SELECT 1 FROM tag_parents WHERE parent_tag = $2 AND child_tag = tag_parents.child_tag)",
    )
    .bind(from_id)
    .bind(into_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM tag_parents WHERE parent_tag = $1")
        .bind(from_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE tag_parents SET child_tag = $2 WHERE child_tag = $1 \
         AND NOT EXISTS (SELECT 1 FROM tag_parents WHERE child_tag = $2 AND parent_tag = tag_parents.parent_tag)",
    )
    .bind(from_id)
    .bind(into_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM tag_parents WHERE child_tag = $1")
        .bind(from_id)
        .execute(&mut *tx)
        .await?;

    // user_interests
    sqlx::query(
        "UPDATE user_interests SET tag_id = $2 WHERE tag_id = $1 \
         AND did NOT IN (SELECT did FROM user_interests WHERE tag_id = $2)",
    )
    .bind(from_id)
    .bind(into_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM user_interests WHERE tag_id = $1")
        .bind(from_id)
        .execute(&mut *tx)
        .await?;

    // Delete the source tag
    sqlx::query("DELETE FROM tags WHERE id = $1")
        .bind(from_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Search tags by prefix/substring match on id and name.
pub async fn search_tags(pool: &PgPool, query: &str, limit: i64) -> Result<Vec<Tag>> {
    let pattern = format!("%{query}%");
    // Return every tag whose id/name/aliases matches the query, without
    // collapsing siblings. A zh-typing user querying "线性" sees
    // "线性代数" / "线性逻辑" (zh siblings that match); an en-typing
    // user querying "linear" sees "Linear Algebra" / "Linear Logic"
    // (en siblings). If the user's query matches across languages the
    // autocomplete will list each match separately — they pick the
    // label they want to use.
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT t.id, t.name, t.names, t.description, t.created_by, t.created_at, t.group_id, t.lang \
         FROM tags t \
         WHERE t.removed_at IS NULL AND ( \
             t.id ILIKE $1 OR t.name ILIKE $1 OR t.names::text ILIKE $1 \
             OR EXISTS (SELECT 1 FROM tag_aliases a WHERE a.tag_id = t.id AND a.alias ILIKE $1) \
         ) \
         ORDER BY \
             CASE WHEN t.id = $2 THEN 0 WHEN t.id ILIKE $3 THEN 1 ELSE 2 END, \
             t.name \
         LIMIT $4",
    )
    .bind(&pattern)
    .bind(query)
    .bind(format!("{query}%"))
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(tags)
}

/// Batch-fetch tag i18n names for a set of IDs. Returns id -> { locale -> name }.
pub async fn get_tag_names_i18n(
    pool: &PgPool,
    tag_ids: &[String],
) -> Result<HashMap<String, HashMap<String, String>>> {
    if tag_ids.is_empty() {
        return Ok(HashMap::new());
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        id: String,
        names: sqlx::types::Json<HashMap<String, String>>,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT id, names FROM tags WHERE id = ANY($1)",
    )
    .bind(tag_ids)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| (r.id, r.names.0)).collect())
}

// ── Aliases ────────────────────────────────────────────────────────────

pub async fn add_alias(pool: &PgPool, alias: &str, tag_id: &str) -> crate::Result<()> {
    sqlx::query("INSERT INTO tag_aliases (alias, tag_id) VALUES ($1, $2) ON CONFLICT (alias) DO UPDATE SET tag_id = $2")
        .bind(alias).bind(tag_id)
        .execute(pool).await?;
    Ok(())
}

pub async fn remove_alias(pool: &PgPool, alias: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM tag_aliases WHERE alias = $1")
        .bind(alias).execute(pool).await?;
    Ok(())
}

pub async fn list_aliases(pool: &PgPool, tag_id: &str) -> crate::Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT alias FROM tag_aliases WHERE tag_id = $1 ORDER BY alias")
        .bind(tag_id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Derive the set of "topic" tags for a content — the transitive closure of
/// parents (in the global tag_parents hierarchy) of the content's teach
/// tags. Result excludes the teach tags themselves.
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
    .bind(content_uri)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Resolve a tag ID or alias to the canonical tag ID.
pub async fn resolve_tag(pool: &PgPool, id_or_alias: &str) -> crate::Result<String> {
    // First check if it's a direct tag ID
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tags WHERE id = $1)")
        .bind(id_or_alias).fetch_one(pool).await?;
    if exists {
        return Ok(id_or_alias.to_string());
    }
    // Then check aliases
    let canonical: Option<String> = sqlx::query_scalar("SELECT tag_id FROM tag_aliases WHERE alias = $1")
        .bind(id_or_alias).fetch_optional(pool).await?;
    canonical.ok_or_else(|| crate::Error::NotFound { entity: "tag", id: id_or_alias.to_string() })
}

// ── Alias / translation groups ────────────────────────────────────────

/// Return every tag id that lives in the same alias/translation group as
/// the given tag (including the tag itself). Used to treat same-concept
/// tags as a single unit when querying edge tables.
pub async fn list_group_members(pool: &PgPool, tag_id: &str) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM tags WHERE group_id = (SELECT group_id FROM tags WHERE id = $1) \
         ORDER BY (lang = 'en') DESC, lang, id",
    )
    .bind(tag_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Return every tag in the same group as `tag_id` with its lang. Ordered
/// English-first, then by locale. Useful for UI that shows the full group.
pub async fn list_group_siblings(pool: &PgPool, tag_id: &str) -> Result<Vec<Tag>> {
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT id, name, names, description, created_by, created_at, group_id, lang FROM tags \
         WHERE group_id = (SELECT group_id FROM tags WHERE id = $1) \
         ORDER BY (lang = 'en') DESC, lang, id",
    )
    .bind(tag_id)
    .fetch_all(pool)
    .await?;
    Ok(tags)
}

/// Given a list of tag ids (possibly containing same-group duplicates),
/// return a deduplicated list — one representative per group. The
/// representative is English-preferred, then the caller-supplied order.
pub async fn dedupe_by_group(pool: &PgPool, tag_ids: &[String]) -> Result<Vec<String>> {
    if tag_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, group_id FROM tags WHERE id = ANY($1)",
    )
    .bind(tag_ids)
    .fetch_all(pool)
    .await?;

    // Keep the first id per group in input order, preferring the one
    // whose input-position is smallest and whose lang is en if multiple.
    use std::collections::HashMap;
    let mut group_of: HashMap<String, String> = HashMap::new();
    for (id, g) in &rows { group_of.insert(id.clone(), g.clone()); }
    let mut seen_groups: HashMap<String, String> = HashMap::new();
    let mut out = Vec::new();
    for id in tag_ids {
        if let Some(g) = group_of.get(id) {
            if !seen_groups.contains_key(g) {
                seen_groups.insert(g.clone(), id.clone());
                out.push(id.clone());
            }
        } else {
            out.push(id.clone()); // tag not found — keep as-is
        }
    }
    Ok(out)
}

/// Set a group's representative for the given member's language. Promoting
/// sibling X makes X the canonical label for lang(X) in this group — the
/// UI will display it whenever a reader's locale matches lang(X). Other
/// languages' reps are untouched.
pub async fn set_group_representative(
    pool: &PgPool,
    anchor_tag_id: &str,
    member_id: &str,
) -> Result<()> {
    // Validate both tags are in the same group; read member's lang.
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT anchor.group_id, m.lang \
         FROM tags anchor JOIN tags m ON m.group_id = anchor.group_id \
         WHERE anchor.id = $1 AND m.id = $2",
    )
    .bind(anchor_tag_id)
    .bind(member_id)
    .fetch_optional(pool)
    .await?;
    let (group_id, lang) = row.ok_or_else(|| crate::Error::Validation(vec![
        crate::validation::ValidationError {
            field: "member_id".into(),
            message: "member is not in the same group as anchor".into(),
        }
    ]))?;
    sqlx::query(
        "INSERT INTO tag_group_representatives (group_id, lang, tag_id) VALUES ($1, $2, $3) \
         ON CONFLICT (group_id, lang) DO UPDATE SET tag_id = EXCLUDED.tag_id",
    )
    .bind(&group_id)
    .bind(&lang)
    .bind(member_id)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Deletion requests ────────────────────────────────────────────────

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

/// Record a user's request to remove a tag. Admin must later approve
/// or reject; the tag isn't touched until approval. Rejects if there's
/// already a pending request on the tag (by any user) so a single
/// pending review is the canonical "under review" state.
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
    .bind(tag_id)
    .fetch_one(pool)
    .await?;
    if existing {
        return Err(crate::Error::Validation(vec![
            crate::validation::ValidationError {
                field: "tag_id".into(),
                message: "this tag already has a pending deletion request under review".into(),
            }
        ]));
    }
    let row = sqlx::query_as::<_, TagDeletionRequest>(
        "INSERT INTO tag_deletion_requests (tag_id, requester_did, reason) \
         VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(tag_id)
    .bind(requester_did)
    .bind(reason)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Is there a pending deletion request on this tag? Used by the UI to
/// show an "under review" banner instead of the submit form.
pub async fn has_pending_deletion(pool: &PgPool, tag_id: &str) -> Result<bool> {
    let existing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tag_deletion_requests \
         WHERE tag_id = $1 AND status = 'pending')",
    )
    .bind(tag_id)
    .fetch_one(pool)
    .await?;
    Ok(existing)
}

pub async fn list_pending_tag_deletions(pool: &PgPool) -> Result<Vec<TagDeletionRequest>> {
    let rows = sqlx::query_as::<_, TagDeletionRequest>(
        "SELECT * FROM tag_deletion_requests WHERE status = 'pending' ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
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
    .bind(request_id)
    .bind(reviewer_did)
    .bind(note)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| crate::Error::NotFound { entity: "tag_deletion_request", id: request_id.to_string() })?;

    hard_delete_tag(&mut tx, &tag_id).await?;
    tx.commit().await?;
    Ok(())
}

/// Remove every reference to a tag from edge tables that don't have
/// ON DELETE CASCADE wired up, then delete the tag row. If the tag was
/// the last member of its group, the empty group row is dropped too.
async fn hard_delete_tag(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tag_id: &str,
) -> Result<()> {
    // Non-cascading FK tables — clean up first.
    for table in &[
        "content_prereqs",
        "content_teaches",
        "content_topics",
        "course_tags",
        "listing_preferred_tags",
        "listing_required_tags",
        "user_interests",
        "user_skills",
        "skill_trees",
    ] {
        sqlx::query(&format!("DELETE FROM {table} WHERE tag_id = $1"))
            .bind(tag_id)
            .execute(&mut **tx)
            .await?;
    }
    // parent/child pairs
    for (table, cols) in &[
        ("user_tag_tree", ("parent_tag", "child_tag")),
        ("skill_tree_edges", ("parent_tag", "child_tag")),
        ("tag_parent_edits", ("parent_tag", "child_tag")),
        ("skill_tree_prereqs", ("from_tag", "to_tag")),
        ("user_tag_prereqs", ("from_tag", "to_tag")),
    ] {
        let (a, b) = cols;
        sqlx::query(&format!("DELETE FROM {table} WHERE {a} = $1 OR {b} = $1"))
            .bind(tag_id)
            .execute(&mut **tx)
            .await?;
    }

    let group_id: Option<String> = sqlx::query_scalar("SELECT group_id FROM tags WHERE id = $1")
        .bind(tag_id)
        .fetch_optional(&mut **tx)
        .await?;

    // Delete the tag — cascading FKs (tag_parents, tag_aliases,
    // tag_group_representatives, course_session_*, tag_deletion_requests)
    // clean themselves up.
    sqlx::query("DELETE FROM tags WHERE id = $1")
        .bind(tag_id)
        .execute(&mut **tx)
        .await?;

    // If group is now empty, drop it (and the stale rep FK that
    // cascaded from the tag row).
    if let Some(g) = group_id {
        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tags WHERE group_id = $1",
        )
        .bind(&g)
        .fetch_one(&mut **tx)
        .await?;
        if remaining == 0 {
            sqlx::query("DELETE FROM tag_groups WHERE id = $1")
                .bind(&g)
                .execute(&mut **tx)
                .await?;
        }
    }
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
    .bind(request_id)
    .bind(reviewer_did)
    .bind(note)
    .execute(pool)
    .await?;
    Ok(())
}

/// Merge the group containing `drop_tag_id` into the group containing
/// `keep_tag_id`. All members of the drop-group move to the keep-group;
/// representatives merge (keep-group's per-lang reps take precedence,
/// drop-group's reps fill in missing langs). The emptied group row is
/// deleted.
pub async fn merge_groups(pool: &PgPool, keep_tag_id: &str, drop_tag_id: &str) -> Result<()> {
    let keep_group: String = sqlx::query_scalar("SELECT group_id FROM tags WHERE id = $1")
        .bind(keep_tag_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "tag", id: keep_tag_id.to_string() })?;
    let drop_group: String = sqlx::query_scalar("SELECT group_id FROM tags WHERE id = $1")
        .bind(drop_tag_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "tag", id: drop_tag_id.to_string() })?;
    if keep_group == drop_group {
        return Ok(()); // already siblings; no-op
    }

    let mut tx = pool.begin().await?;
    // Move every member of drop_group into keep_group.
    sqlx::query("UPDATE tags SET group_id = $1 WHERE group_id = $2")
        .bind(&keep_group)
        .bind(&drop_group)
        .execute(&mut *tx)
        .await?;
    // Keep-group's per-lang reps win; drop-group's fill in where keep
    // didn't have one.
    sqlx::query(
        "INSERT INTO tag_group_representatives (group_id, lang, tag_id) \
         SELECT $1, lang, tag_id FROM tag_group_representatives WHERE group_id = $2 \
         ON CONFLICT (group_id, lang) DO NOTHING",
    )
    .bind(&keep_group)
    .bind(&drop_group)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM tag_group_representatives WHERE group_id = $1")
        .bind(&drop_group)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM tag_groups WHERE id = $1")
        .bind(&drop_group)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Fetch the (lang → representative tag id) mapping for a group, keyed
/// off any member.
pub async fn list_group_representatives(
    pool: &PgPool,
    anchor_tag_id: &str,
) -> Result<std::collections::HashMap<String, String>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT r.lang, r.tag_id FROM tag_group_representatives r \
         WHERE r.group_id = (SELECT group_id FROM tags WHERE id = $1)",
    )
    .bind(anchor_tag_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// Add a sibling tag to the same alias/translation group as an existing
/// tag. The new tag gets its own id/name/lang but shares group_id with the
/// anchor.
pub async fn add_group_member(
    pool: &PgPool,
    anchor_tag_id: &str,
    new_id: &str,
    new_name: &str,
    lang: &str,
    created_by: &str,
) -> Result<Tag> {
    let group_id: String = sqlx::query_scalar("SELECT group_id FROM tags WHERE id = $1")
        .bind(anchor_tag_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "tag", id: anchor_tag_id.to_string() })?;

    sqlx::query(
        "INSERT INTO tags (id, name, created_by, lang, group_id, names) \
         VALUES ($1, $2, $3, $4, $5, jsonb_build_object($4::text, $2::text))",
    )
    .bind(new_id)
    .bind(new_name)
    .bind(created_by)
    .bind(lang)
    .bind(&group_id)
    .execute(pool)
    .await?;
    // If this lang has no representative yet, the new tag is the only
    // candidate — mark it as the rep automatically. Admin can later
    // promote a different sibling if they add a better label.
    sqlx::query(
        "INSERT INTO tag_group_representatives (group_id, lang, tag_id) \
         VALUES ($1, $2, $3) ON CONFLICT (group_id, lang) DO NOTHING",
    )
    .bind(&group_id)
    .bind(lang)
    .bind(new_id)
    .execute(pool)
    .await?;
    get_tag(pool, new_id).await
}

/// Remove a tag from its group (deletes the tag row). If the group becomes
/// empty, drop the group row too.
pub async fn remove_group_member(pool: &PgPool, tag_id: &str) -> Result<()> {
    let group_id: Option<String> = sqlx::query_scalar("SELECT group_id FROM tags WHERE id = $1")
        .bind(tag_id)
        .fetch_optional(pool)
        .await?;
    sqlx::query("DELETE FROM tags WHERE id = $1").bind(tag_id).execute(pool).await?;
    if let Some(g) = group_id {
        let still_populated: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tags WHERE group_id = $1",
        )
        .bind(&g)
        .fetch_one(pool)
        .await?;
        if still_populated == 0 {
            sqlx::query("DELETE FROM tag_groups WHERE id = $1").bind(&g).execute(pool).await?;
        }
    }
    Ok(())
}

/// Expand a list of tag ids to include every sibling in each tag's group.
/// Used when querying edge tables ("find all content with prereq X" must
/// also find content with prereq on X's zh-sibling or alias-sibling).
pub async fn expand_to_group(pool: &PgPool, tag_ids: &[String]) -> Result<Vec<String>> {
    if tag_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT id FROM tags \
         WHERE group_id IN (SELECT group_id FROM tags WHERE id = ANY($1))",
    )
    .bind(tag_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}
