use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct TagParent {
    pub parent_tag: String,
    pub child_tag: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TagParentEdit {
    pub id: i64,
    pub parent_tag: String,
    pub child_tag: String,
    pub action: String,
    pub editor_did: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_all(pool: &PgPool) -> Result<Vec<TagParent>> {
    let rows = sqlx::query_as::<_, TagParent>(
        "SELECT parent_tag, child_tag FROM tag_parents ORDER BY parent_tag, child_tag",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn add_edge(
    pool: &PgPool,
    parent_tag: &str,
    child_tag: &str,
    editor_did: &str,
) -> Result<()> {
    if parent_tag == child_tag {
        return Err(crate::Error::Validation(vec![
            crate::validation::ValidationError {
                field: "child_tag".into(),
                message: "cannot equal parent_tag".into(),
            },
        ]));
    }
    let mut tx = pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO tag_parents (parent_tag, child_tag) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(parent_tag)
    .bind(child_tag)
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() > 0 {
        sqlx::query(
            "INSERT INTO tag_parent_edits (parent_tag, child_tag, action, editor_did) \
             VALUES ($1, $2, 'add', $3)",
        )
        .bind(parent_tag)
        .bind(child_tag)
        .bind(editor_did)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn remove_edge(
    pool: &PgPool,
    parent_tag: &str,
    child_tag: &str,
    editor_did: &str,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    let deleted = sqlx::query(
        "DELETE FROM tag_parents WHERE parent_tag = $1 AND child_tag = $2",
    )
    .bind(parent_tag)
    .bind(child_tag)
    .execute(&mut *tx)
    .await?;
    if deleted.rows_affected() > 0 {
        sqlx::query(
            "INSERT INTO tag_parent_edits (parent_tag, child_tag, action, editor_did) \
             VALUES ($1, $2, 'remove', $3)",
        )
        .bind(parent_tag)
        .bind(child_tag)
        .bind(editor_did)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
