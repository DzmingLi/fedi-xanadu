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
