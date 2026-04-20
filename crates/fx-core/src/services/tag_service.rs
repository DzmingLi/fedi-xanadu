use std::collections::HashMap;

use sqlx::PgPool;

use crate::models::{CreateTag, Tag};
use crate::Result;

pub async fn list_tags(pool: &PgPool, limit: i64) -> Result<Vec<Tag>> {
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT id, name, names, description, created_by, created_at, group_id, lang FROM tags ORDER BY name LIMIT $1",
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
pub async fn ensure_tag<'e, E>(executor: E, tag_id: &str, created_by: &str) -> Result<()>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query(
        "INSERT INTO tags (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
    )
    .bind(tag_id)
    .bind(tag_id)
    .bind(created_by)
    .execute(executor)
    .await?;
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
    // De-dupe by group: when several tags in the same alias/translation
    // group match the query, return only one representative (preferring
    // the English label and then the tag whose id equals the query).
    // We achieve this via a DISTINCT ON subquery — `DISTINCT ON (group_id)`
    // keeps the first row per group under the given ORDER BY — and then
    // re-order the result for display.
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT * FROM ( \
             SELECT DISTINCT ON (t.group_id) \
                 t.id, t.name, t.names, t.description, t.created_by, t.created_at, t.group_id, t.lang, \
                 (CASE WHEN t.id = $2 THEN 0 WHEN t.id ILIKE $3 THEN 1 ELSE 2 END) AS rank \
             FROM tags t \
             WHERE t.id ILIKE $1 OR t.name ILIKE $1 OR t.names::text ILIKE $1 \
                OR EXISTS (SELECT 1 FROM tag_aliases a WHERE a.tag_id = t.id AND a.alias ILIKE $1) \
             ORDER BY t.group_id, \
                      (t.lang = 'en') DESC, \
                      (CASE WHEN t.id = $2 THEN 0 WHEN t.id ILIKE $3 THEN 1 ELSE 2 END), \
                      t.name \
         ) g \
         ORDER BY rank, name \
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

/// Set which tag in a group is the "representative" — the single label
/// used when the UI needs to pick one for display (prereqs, teaches, skill
/// mastery badges, etc.). Must be a tag that's already in the group.
pub async fn set_group_representative(
    pool: &PgPool,
    anchor_tag_id: &str,
    member_id: &str,
) -> Result<()> {
    // Verify both tags exist and share the same group.
    let same_group: Option<String> = sqlx::query_scalar(
        "SELECT anchor.group_id \
         FROM tags anchor JOIN tags m ON m.group_id = anchor.group_id \
         WHERE anchor.id = $1 AND m.id = $2",
    )
    .bind(anchor_tag_id)
    .bind(member_id)
    .fetch_optional(pool)
    .await?;
    let group_id = same_group.ok_or_else(|| crate::Error::Validation(vec![
        crate::validation::ValidationError {
            field: "member_id".into(),
            message: "member is not in the same group as anchor".into(),
        }
    ]))?;
    sqlx::query("UPDATE tag_groups SET representative_tag_id = $2 WHERE id = $1")
        .bind(&group_id)
        .bind(member_id)
        .execute(pool)
        .await?;
    Ok(())
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
