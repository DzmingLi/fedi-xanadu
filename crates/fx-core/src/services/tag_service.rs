use std::collections::HashMap;

use sqlx::PgPool;

use crate::models::{CreateTag, Tag};
use crate::Result;

pub async fn list_tags(pool: &PgPool, limit: i64) -> Result<Vec<Tag>> {
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT id, name, tag_label_map(tag_id) AS names, description, created_by, created_at, tag_id, lang \
         FROM tag_labels WHERE removed_at IS NULL ORDER BY name LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(tags)
}

pub async fn get_tag(pool: &PgPool, id: &str) -> Result<Tag> {
    sqlx::query_as::<_, Tag>(
        "SELECT id, name, tag_label_map(tag_id) AS names, description, created_by, created_at, tag_id, lang FROM tag_labels WHERE id = $1",
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
    let own_lang = guess_lang_from_id(&input.id);
    let mut tx = pool.begin().await?;

    let new_tag_id: String = sqlx::query_scalar("INSERT INTO tags DEFAULT VALUES RETURNING id")
        .fetch_one(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO tag_labels (id, name, description, created_by, lang, tag_id) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&input.id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(created_by)
    .bind(own_lang)
    .bind(&new_tag_id)
    .execute(&mut *tx)
    .await?;

    // Each translation supplied by the client becomes a sibling label in
    // the same tag. The origin lang is already covered above; skip
    // entries whose value collides with an existing label id to keep the
    // insert idempotent against concurrent writers.
    if let Some(names) = &input.names {
        for (lang, name) in names.iter() {
            let name = name.trim();
            if name.is_empty() || lang == own_lang {
                continue;
            }
            sqlx::query(
                "INSERT INTO tag_labels (id, name, created_by, lang, tag_id) \
                 VALUES ($1, $1, $2, $3, $4) ON CONFLICT (id) DO NOTHING",
            )
            .bind(name)
            .bind(created_by)
            .bind(lang)
            .bind(&new_tag_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
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

/// Ensure a label row exists for `label_id`. If missing, create a fresh
/// tag (alias group) containing just this one label, guess its lang from
/// the id (CJK → `zh`, else `en`), and mark it as the tag's rep for that
/// lang. `INSERT ... ON CONFLICT DO NOTHING` still applies, so callers
/// can safely invoke this for labels they suspect may already exist.
pub async fn ensure_tag(
    conn: &mut sqlx::PgConnection,
    label_id: &str,
    created_by: &str,
) -> Result<()> {
    // Fast path: label already exists.
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tag_labels WHERE id = $1)")
        .bind(label_id)
        .fetch_one(&mut *conn)
        .await?;
    if exists {
        return Ok(());
    }
    let lang = guess_lang_from_id(label_id);
    // Three statements — caller may already be inside a transaction
    // (skill-tree import builds edges in one tx), so we just run
    // sequentially instead of opening a nested one.
    let new_tag_id: String = sqlx::query_scalar(
        "INSERT INTO tags DEFAULT VALUES RETURNING id",
    )
    .fetch_one(&mut *conn)
    .await?;
    let inserted = sqlx::query(
        "INSERT INTO tag_labels (id, name, created_by, lang, tag_id) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(label_id)
    .bind(label_id)
    .bind(created_by)
    .bind(lang)
    .bind(&new_tag_id)
    .execute(&mut *conn)
    .await?
    .rows_affected();
    if inserted > 0 {
        sqlx::query(
            "INSERT INTO tag_representatives (tag_id, lang, label_id) \
             VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(&new_tag_id)
        .bind(lang)
        .bind(label_id)
        .execute(&mut *conn)
        .await?;
    } else {
        // Race: some other inserter put the label in between our check
        // and our INSERT. Clean up the stray tag row.
        sqlx::query("DELETE FROM tags WHERE id = $1")
            .bind(&new_tag_id)
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
        "SELECT id, name FROM tag_labels WHERE id = ANY($1)",
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

/// Update the translations for a label by materializing each per-language
/// entry as a sibling label row in the same tag. The translation map is
/// derived at read time via `tag_label_map`, so no jsonb cache needs
/// updating.
pub async fn update_tag_names(
    pool: &PgPool,
    label_id: &str,
    names: &HashMap<String, String>,
) -> Result<Tag> {
    let origin = get_tag(pool, label_id).await?;
    let mut tx = pool.begin().await?;

    for (lang, name) in names.iter() {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if lang == &origin.lang {
            // Rename the origin label's display name in place; keep its id.
            sqlx::query("UPDATE tag_labels SET name = $1 WHERE id = $2")
                .bind(name)
                .bind(label_id)
                .execute(&mut *tx)
                .await?;
            continue;
        }
        let sibling_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM tag_labels WHERE tag_id = $1 AND lang = $2 LIMIT 1",
        )
        .bind(&origin.tag_id)
        .bind(lang)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(sid) = sibling_id {
            sqlx::query("UPDATE tag_labels SET name = $1 WHERE id = $2")
                .bind(name)
                .bind(&sid)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query(
                "INSERT INTO tag_labels (id, name, lang, tag_id, created_by) \
                 VALUES ($1, $1, $2, $3, $4) \
                 ON CONFLICT (id) DO NOTHING",
            )
            .bind(name)
            .bind(lang)
            .bind(&origin.tag_id)
            .bind(&origin.created_by)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    get_tag(pool, label_id).await
}

/// Merge label `from_id` into label `into_id`. If the two labels belong
/// to different tags, move `from_id`'s tag into `into_id`'s tag first
/// (all content/edge FKs are tag-level, so they collapse automatically).
/// Then delete the source label. If its tag ends up empty, drop the tag
/// too (cascade deletes any remaining FKs).
pub async fn merge_tag(pool: &PgPool, from_id: &str, into_id: &str) -> Result<()> {
    let mut tx = pool.begin().await?;

    let from_tag: String = sqlx::query_scalar("SELECT tag_id FROM tag_labels WHERE id = $1")
        .bind(from_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "tag", id: from_id.to_string() })?;
    let into_tag: String = sqlx::query_scalar("SELECT tag_id FROM tag_labels WHERE id = $1")
        .bind(into_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "tag", id: into_id.to_string() })?;

    if from_tag != into_tag {
        // Move every label of from_tag into into_tag.
        sqlx::query("UPDATE tag_labels SET tag_id = $1 WHERE tag_id = $2")
            .bind(&into_tag)
            .bind(&from_tag)
            .execute(&mut *tx)
            .await?;
        // Keep into_tag's per-lang reps; into_tag's rep wins, from_tag's
        // fills in missing languages.
        sqlx::query(
            "INSERT INTO tag_representatives (tag_id, lang, label_id) \
             SELECT $1, lang, label_id FROM tag_representatives WHERE tag_id = $2 \
             ON CONFLICT (tag_id, lang) DO NOTHING",
        )
        .bind(&into_tag)
        .bind(&from_tag)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM tag_representatives WHERE tag_id = $1")
            .bind(&from_tag)
            .execute(&mut *tx)
            .await?;
        // Delete the (now orphan) from_tag; tag_parents, user_tag_tree,
        // skill_tree_edges, etc. cascade-clean since their tag_id FK
        // references `tags(id) ON DELETE CASCADE`.
        sqlx::query("DELETE FROM tags WHERE id = $1")
            .bind(&from_tag)
            .execute(&mut *tx)
            .await?;
    }

    // Delete the source label itself.
    sqlx::query("DELETE FROM tag_labels WHERE id = $1")
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
        "SELECT t.id, t.name, tag_label_map(t.tag_id) AS names, t.description, t.created_by, t.created_at, t.tag_id, t.lang \
         FROM tag_labels t \
         WHERE t.removed_at IS NULL AND ( \
             t.id ILIKE $1 OR t.name ILIKE $1 \
             OR EXISTS (SELECT 1 FROM tag_labels sib WHERE sib.tag_id = t.tag_id AND sib.name ILIKE $1) \
             OR EXISTS (SELECT 1 FROM tag_aliases a WHERE a.label_id = t.id AND a.alias ILIKE $1) \
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

/// Batch-fetch per-label translation maps for a set of label IDs. Returns
/// label_id -> { locale -> name }. Each label in a tag shares the same
/// translation map (aggregated from the tag's siblings).
pub async fn get_tag_names_i18n(
    pool: &PgPool,
    label_ids: &[String],
) -> Result<HashMap<String, HashMap<String, String>>> {
    if label_ids.is_empty() {
        return Ok(HashMap::new());
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        id: String,
        names: sqlx::types::Json<HashMap<String, String>>,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT id, tag_label_map(tag_id) AS names FROM tag_labels WHERE id = ANY($1)",
    )
    .bind(label_ids)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| (r.id, r.names.0)).collect())
}

// ── Aliases ────────────────────────────────────────────────────────────

pub async fn add_alias(pool: &PgPool, alias: &str, label_id: &str) -> crate::Result<()> {
    sqlx::query("INSERT INTO tag_aliases (alias, label_id) VALUES ($1, $2) ON CONFLICT (alias) DO UPDATE SET label_id = $2")
        .bind(alias).bind(label_id)
        .execute(pool).await?;
    Ok(())
}

pub async fn remove_alias(pool: &PgPool, alias: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM tag_aliases WHERE alias = $1")
        .bind(alias).execute(pool).await?;
    Ok(())
}

pub async fn list_aliases(pool: &PgPool, label_id: &str) -> crate::Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT alias FROM tag_aliases WHERE label_id = $1 ORDER BY alias")
        .bind(label_id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Derive the set of "topic" tags for a content — the transitive closure
/// of parents (in the global `tag_parents` hierarchy) of the content's
/// teach tags. Result excludes the teach tags themselves.
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

/// Resolve a label id or alias to a canonical label id.
pub async fn resolve_tag(pool: &PgPool, id_or_alias: &str) -> crate::Result<String> {
    // First check if it's a direct label id
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tag_labels WHERE id = $1)")
        .bind(id_or_alias).fetch_one(pool).await?;
    if exists {
        return Ok(id_or_alias.to_string());
    }
    // Then check aliases
    let canonical: Option<String> = sqlx::query_scalar("SELECT label_id FROM tag_aliases WHERE alias = $1")
        .bind(id_or_alias).fetch_optional(pool).await?;
    canonical.ok_or_else(|| crate::Error::NotFound { entity: "tag", id: id_or_alias.to_string() })
}

// ── Alias / translation groups ────────────────────────────────────────

/// Return every label id that lives in the same tag (alias/translation
/// group) as the given label, itself included. Used to treat same-concept
/// labels as a single unit when querying edge tables.
pub async fn list_group_members(pool: &PgPool, label_id: &str) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM tag_labels WHERE tag_id = (SELECT tag_id FROM tag_labels WHERE id = $1) \
         ORDER BY (lang = 'en') DESC, lang, id",
    )
    .bind(label_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Return every label sharing the same tag as `label_id`, with its lang.
/// Ordered English-first, then by locale. Useful for UI that shows every
/// language variant.
pub async fn list_group_siblings(pool: &PgPool, label_id: &str) -> Result<Vec<Tag>> {
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT id, name, tag_label_map(tag_id) AS names, description, created_by, created_at, tag_id, lang \
         FROM tag_labels \
         WHERE tag_id = (SELECT tag_id FROM tag_labels WHERE id = $1) \
         ORDER BY (lang = 'en') DESC, lang, id",
    )
    .bind(label_id)
    .fetch_all(pool)
    .await?;
    Ok(tags)
}

/// Given a list of label ids (possibly sharing a tag), return a
/// deduplicated list — one label per tag, preserving input order.
pub async fn dedupe_by_group(pool: &PgPool, label_ids: &[String]) -> Result<Vec<String>> {
    if label_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, tag_id FROM tag_labels WHERE id = ANY($1)",
    )
    .bind(label_ids)
    .fetch_all(pool)
    .await?;

    use std::collections::HashMap;
    let mut tag_of: HashMap<String, String> = HashMap::new();
    for (id, t) in &rows { tag_of.insert(id.clone(), t.clone()); }
    let mut seen_tags: HashMap<String, String> = HashMap::new();
    let mut out = Vec::new();
    for id in label_ids {
        if let Some(t) = tag_of.get(id) {
            if !seen_tags.contains_key(t) {
                seen_tags.insert(t.clone(), id.clone());
                out.push(id.clone());
            }
        } else {
            out.push(id.clone()); // label not found — keep as-is
        }
    }
    Ok(out)
}

/// Set a tag's canonical label for the given sibling's language.
/// Promoting sibling X makes X the canonical label for lang(X) — the UI
/// will display it whenever a reader's locale matches. Other langs'
/// reps are untouched.
pub async fn set_group_representative(
    pool: &PgPool,
    anchor_label_id: &str,
    member_id: &str,
) -> Result<()> {
    // Validate both labels are in the same tag; read member's lang.
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT anchor.tag_id, m.lang \
         FROM tag_labels anchor JOIN tag_labels m ON m.tag_id = anchor.tag_id \
         WHERE anchor.id = $1 AND m.id = $2",
    )
    .bind(anchor_label_id)
    .bind(member_id)
    .fetch_optional(pool)
    .await?;
    let (tag_id, lang) = row.ok_or_else(|| crate::Error::Validation(vec![
        crate::validation::ValidationError {
            field: "member_id".into(),
            message: "member is not in the same tag as anchor".into(),
        }
    ]))?;
    sqlx::query(
        "INSERT INTO tag_representatives (tag_id, lang, label_id) VALUES ($1, $2, $3) \
         ON CONFLICT (tag_id, lang) DO UPDATE SET label_id = EXCLUDED.label_id",
    )
    .bind(&tag_id)
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
    pub label_id: String,
    pub requester_did: String,
    pub reason: String,
    pub status: String,
    pub reviewer_did: Option<String>,
    pub review_note: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Record a user's request to remove a label. Admin must later approve
/// or reject; the label isn't touched until approval. Rejects if there's
/// already a pending request on the same label so a single pending
/// review is the canonical "under review" state.
pub async fn request_tag_deletion(
    pool: &PgPool,
    label_id: &str,
    requester_did: &str,
    reason: &str,
) -> Result<TagDeletionRequest> {
    let existing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tag_deletion_requests \
         WHERE label_id = $1 AND status = 'pending')",
    )
    .bind(label_id)
    .fetch_one(pool)
    .await?;
    if existing {
        return Err(crate::Error::Validation(vec![
            crate::validation::ValidationError {
                field: "label_id".into(),
                message: "this label already has a pending deletion request under review".into(),
            }
        ]));
    }
    let row = sqlx::query_as::<_, TagDeletionRequest>(
        "INSERT INTO tag_deletion_requests (label_id, requester_did, reason) \
         VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(label_id)
    .bind(requester_did)
    .bind(reason)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Is there a pending deletion request on this label? Used by the UI to
/// show an "under review" banner instead of the submit form.
pub async fn has_pending_deletion(pool: &PgPool, label_id: &str) -> Result<bool> {
    let existing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tag_deletion_requests \
         WHERE label_id = $1 AND status = 'pending')",
    )
    .bind(label_id)
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
    let label_id: String = sqlx::query_scalar(
        "UPDATE tag_deletion_requests \
         SET status = 'approved', reviewer_did = $2, review_note = $3, reviewed_at = NOW() \
         WHERE id = $1 AND status = 'pending' RETURNING label_id",
    )
    .bind(request_id)
    .bind(reviewer_did)
    .bind(note)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| crate::Error::NotFound { entity: "tag_deletion_request", id: request_id.to_string() })?;

    hard_delete_tag(&mut tx, &label_id).await?;
    tx.commit().await?;
    Ok(())
}

/// Delete the label row. If it was the last label in its tag, drop the
/// tag too — which cascades to every tag-level FK (content_teaches,
/// content_prereqs, tag_parents, skill_tree_edges, user_skills, …).
/// Individual content/edge FKs point at the *tag*, so removing a single
/// label while siblings remain is non-destructive: those FKs stay
/// pointed at a still-valid tag.
async fn hard_delete_tag(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    label_id: &str,
) -> Result<()> {
    let tag_id: Option<String> = sqlx::query_scalar("SELECT tag_id FROM tag_labels WHERE id = $1")
        .bind(label_id)
        .fetch_optional(&mut **tx)
        .await?;

    sqlx::query("DELETE FROM tag_labels WHERE id = $1")
        .bind(label_id)
        .execute(&mut **tx)
        .await?;

    if let Some(t) = tag_id {
        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tag_labels WHERE tag_id = $1",
        )
        .bind(&t)
        .fetch_one(&mut **tx)
        .await?;
        if remaining == 0 {
            sqlx::query("DELETE FROM tags WHERE id = $1")
                .bind(&t)
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

/// Merge the tag containing `drop_label_id` into the tag containing
/// `keep_label_id`. All labels of the drop-tag move to the keep-tag;
/// representatives merge (keep-tag's per-lang reps take precedence,
/// drop-tag's reps fill in missing langs). The emptied tag row is
/// deleted.
pub async fn merge_groups(pool: &PgPool, keep_label_id: &str, drop_label_id: &str) -> Result<()> {
    let keep_tag: String = sqlx::query_scalar("SELECT tag_id FROM tag_labels WHERE id = $1")
        .bind(keep_label_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "tag", id: keep_label_id.to_string() })?;
    let drop_tag: String = sqlx::query_scalar("SELECT tag_id FROM tag_labels WHERE id = $1")
        .bind(drop_label_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "tag", id: drop_label_id.to_string() })?;
    if keep_tag == drop_tag {
        return Ok(()); // already siblings; no-op
    }

    let mut tx = pool.begin().await?;
    // Move every label of drop_tag into keep_tag.
    sqlx::query("UPDATE tag_labels SET tag_id = $1 WHERE tag_id = $2")
        .bind(&keep_tag)
        .bind(&drop_tag)
        .execute(&mut *tx)
        .await?;
    // Keep-tag's per-lang reps win; drop-tag's fill in where keep didn't
    // have one.
    sqlx::query(
        "INSERT INTO tag_representatives (tag_id, lang, label_id) \
         SELECT $1, lang, label_id FROM tag_representatives WHERE tag_id = $2 \
         ON CONFLICT (tag_id, lang) DO NOTHING",
    )
    .bind(&keep_tag)
    .bind(&drop_tag)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM tag_representatives WHERE tag_id = $1")
        .bind(&drop_tag)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM tags WHERE id = $1")
        .bind(&drop_tag)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Fetch the (lang → canonical label id) mapping for a tag, keyed off any
/// label in that tag.
pub async fn list_group_representatives(
    pool: &PgPool,
    anchor_label_id: &str,
) -> Result<std::collections::HashMap<String, String>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT r.lang, r.label_id FROM tag_representatives r \
         WHERE r.tag_id = (SELECT tag_id FROM tag_labels WHERE id = $1)",
    )
    .bind(anchor_label_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// Add a sibling label to the same tag as an existing label. The new label
/// gets its own id/name/lang but shares tag_id with the anchor.
pub async fn add_group_member(
    pool: &PgPool,
    anchor_label_id: &str,
    new_id: &str,
    new_name: &str,
    lang: &str,
    created_by: &str,
) -> Result<Tag> {
    let tag_id: String = sqlx::query_scalar("SELECT tag_id FROM tag_labels WHERE id = $1")
        .bind(anchor_label_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "tag", id: anchor_label_id.to_string() })?;

    sqlx::query(
        "INSERT INTO tag_labels (id, name, created_by, lang, tag_id) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(new_id)
    .bind(new_name)
    .bind(created_by)
    .bind(lang)
    .bind(&tag_id)
    .execute(pool)
    .await?;
    // If this lang has no representative yet, the new label is the only
    // candidate — mark it as the rep automatically. Admin can later
    // promote a different sibling if they add a better label.
    sqlx::query(
        "INSERT INTO tag_representatives (tag_id, lang, label_id) \
         VALUES ($1, $2, $3) ON CONFLICT (tag_id, lang) DO NOTHING",
    )
    .bind(&tag_id)
    .bind(lang)
    .bind(new_id)
    .execute(pool)
    .await?;
    get_tag(pool, new_id).await
}

/// Remove a label from its tag. If the tag becomes empty, drop it too.
pub async fn remove_group_member(pool: &PgPool, label_id: &str) -> Result<()> {
    let tag_id: Option<String> = sqlx::query_scalar("SELECT tag_id FROM tag_labels WHERE id = $1")
        .bind(label_id)
        .fetch_optional(pool)
        .await?;
    sqlx::query("DELETE FROM tag_labels WHERE id = $1").bind(label_id).execute(pool).await?;
    if let Some(t) = tag_id {
        let still_populated: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tag_labels WHERE tag_id = $1",
        )
        .bind(&t)
        .fetch_one(pool)
        .await?;
        if still_populated == 0 {
            sqlx::query("DELETE FROM tags WHERE id = $1").bind(&t).execute(pool).await?;
        }
    }
    Ok(())
}

/// Expand a list of label ids to include every sibling in each label's tag.
/// Used when querying edge tables ("find all content with prereq X" must
/// also find content with prereq on X's zh-sibling or alias-sibling).
pub async fn expand_to_group(pool: &PgPool, label_ids: &[String]) -> Result<Vec<String>> {
    if label_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT id FROM tag_labels \
         WHERE tag_id IN (SELECT tag_id FROM tag_labels WHERE id = ANY($1))",
    )
    .bind(label_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}
