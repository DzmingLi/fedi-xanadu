use std::collections::HashMap;

use sqlx::PgPool;

use crate::models::{CreateTag, Tag};
use crate::Result;

pub async fn list_tags(pool: &PgPool, limit: i64) -> Result<Vec<Tag>> {
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT id, name, names, description, created_by, created_at FROM tags ORDER BY name LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(tags)
}

pub async fn get_tag(pool: &PgPool, id: &str) -> Result<Tag> {
    sqlx::query_as::<_, Tag>(
        "SELECT id, name, names, description, created_by, created_at FROM tags WHERE id = $1",
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

    // user_tag_tree
    sqlx::query("UPDATE user_tag_tree SET parent_tag = $2 WHERE parent_tag = $1")
        .bind(from_id)
        .bind(into_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE user_tag_tree SET child_tag = $2 WHERE child_tag = $1")
        .bind(from_id)
        .bind(into_id)
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
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT id, name, names, description, created_by, created_at FROM tags \
         WHERE id ILIKE $1 OR name ILIKE $1 \
         ORDER BY CASE WHEN id = $2 THEN 0 WHEN id ILIKE $3 THEN 1 ELSE 2 END, name \
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
