use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

use crate::error::Error;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SeriesRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    pub order_index: i32,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SeriesListRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    pub order_index: i32,
    pub created_by: String,
    pub author_handle: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SeriesArticleRow {
    pub series_id: String,
    pub article_uri: String,
    pub title: String,
    pub description: String,
    pub lang: String,
    pub order_index: i32,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SeriesPrereqRow {
    pub article_uri: String,
    pub prereq_article_uri: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeriesDetailResponse {
    pub series: SeriesRow,
    pub articles: Vec<SeriesArticleRow>,
    pub prereqs: Vec<SeriesPrereqRow>,
    pub children: Vec<SeriesRow>,
}

/// Recursive tree node for full hierarchy display.
#[derive(Debug, Clone, Serialize)]
pub struct SeriesTreeNode {
    pub series: SeriesRow,
    pub articles: Vec<SeriesArticleRow>,
    pub children: Vec<SeriesTreeNode>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SeriesArticleMemberRow {
    pub series_id: String,
    pub article_uri: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeriesContextItem {
    pub series_id: String,
    pub series_title: String,
    pub total: i64,
    pub prev: Vec<SeriesNavItem>,
    pub next: Vec<SeriesNavItem>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SeriesNavItem {
    pub article_uri: String,
    pub title: String,
}

pub async fn list_series(pool: &PgPool, limit: i64) -> crate::Result<Vec<SeriesListRow>> {
    let rows = sqlx::query_as::<_, SeriesListRow>(
        "SELECT s.id, s.title, s.description, \
                s.parent_id, s.order_index, s.created_by, pu.handle AS author_handle, s.created_at \
         FROM series s \
         LEFT JOIN platform_users pu ON s.created_by = pu.did \
         ORDER BY s.created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn create_series(
    pool: &PgPool,
    id: &str,
    title: &str,
    description: Option<&str>,
    topics: &[String],
    parent_id: Option<&str>,
    created_by: &str,
) -> crate::Result<SeriesRow> {
    // Auto-assign order_index: append after existing siblings
    let order_index: i32 = if parent_id.is_some() {
        sqlx::query_scalar::<_, Option<i32>>(
            "SELECT MAX(order_index) FROM series WHERE parent_id = $1",
        )
        .bind(parent_id)
        .fetch_one(pool)
        .await?
        .unwrap_or(-1)
        + 1
    } else {
        0
    };

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO series (id, title, description, parent_id, order_index, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(title)
    .bind(description)
    .bind(parent_id)
    .bind(order_index)
    .bind(created_by)
    .execute(&mut *tx)
    .await?;

    for topic in topics {
        sqlx::query(
            "INSERT INTO content_topics (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(topic)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let row = sqlx::query_as::<_, SeriesRow>(
        "SELECT id, title, description, parent_id, order_index, created_by, created_at \
         FROM series WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_series_detail(pool: &PgPool, id: &str) -> crate::Result<SeriesDetailResponse> {
    let series = sqlx::query_as::<_, SeriesRow>(
        "SELECT id, title, description, parent_id, order_index, created_by, created_at \
         FROM series WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| Error::NotFound {
        entity: "series",
        id: id.to_string(),
    })?;

    let articles = sqlx::query_as::<_, SeriesArticleRow>(
        "SELECT sa.series_id, sa.article_uri, a.title, COALESCE(a.description, '') AS description, \
                a.lang, sa.order_index \
         FROM series_articles sa JOIN articles a ON sa.article_uri = a.at_uri \
         WHERE sa.series_id = $1 ORDER BY sa.order_index",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    let prereqs = sqlx::query_as::<_, SeriesPrereqRow>(
        "SELECT article_uri, prereq_article_uri FROM series_article_prereqs WHERE series_id = $1",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    let children = sqlx::query_as::<_, SeriesRow>(
        "SELECT id, title, description, parent_id, order_index, created_by, created_at \
         FROM series WHERE parent_id = $1 ORDER BY order_index",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(SeriesDetailResponse {
        series,
        articles,
        prereqs,
        children,
    })
}

pub async fn add_series_article(
    pool: &PgPool,
    series_id: &str,
    article_uri: &str,
) -> crate::Result<()> {
    let order_index: i32 = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(order_index) FROM series_articles WHERE series_id = $1",
    )
    .bind(series_id)
    .fetch_one(pool)
    .await?
    .unwrap_or(-1)
    + 1;

    sqlx::query(
        "INSERT INTO series_articles (series_id, article_uri, order_index) \
         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(series_id)
    .bind(article_uri)
    .bind(order_index)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_series_article(
    pool: &PgPool,
    series_id: &str,
    article_uri: &str,
) -> crate::Result<()> {
    // Also remove prereq edges involving this article
    sqlx::query(
        "DELETE FROM series_article_prereqs WHERE series_id = $1 AND (article_uri = $2 OR prereq_article_uri = $2)",
    )
    .bind(series_id)
    .bind(article_uri)
    .execute(pool)
    .await?;

    sqlx::query("DELETE FROM series_articles WHERE series_id = $1 AND article_uri = $2")
        .bind(series_id)
        .bind(article_uri)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn add_series_prereq(
    pool: &PgPool,
    series_id: &str,
    article_uri: &str,
    prereq_article_uri: &str,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO series_article_prereqs (series_id, article_uri, prereq_article_uri) \
         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(series_id)
    .bind(article_uri)
    .bind(prereq_article_uri)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_series_prereq(
    pool: &PgPool,
    series_id: &str,
    article_uri: &str,
    prereq_article_uri: &str,
) -> crate::Result<()> {
    sqlx::query(
        "DELETE FROM series_article_prereqs \
         WHERE series_id = $1 AND article_uri = $2 AND prereq_article_uri = $3",
    )
    .bind(series_id)
    .bind(article_uri)
    .bind(prereq_article_uri)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_series_owner(pool: &PgPool, series_id: &str) -> crate::Result<String> {
    let owner = sqlx::query_scalar::<_, String>(
        "SELECT created_by FROM series WHERE id = $1",
    )
    .bind(series_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| Error::NotFound {
        entity: "series",
        id: series_id.to_string(),
    })?;
    Ok(owner)
}

pub async fn all_series_articles(pool: &PgPool, limit: i64) -> crate::Result<Vec<SeriesArticleMemberRow>> {
    let rows = sqlx::query_as::<_, SeriesArticleMemberRow>(
        "SELECT series_id, article_uri FROM series_articles LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Walk up parent_id links to find the root series of any (possibly nested) series.
pub async fn get_root_series_id(pool: &PgPool, series_id: &str) -> crate::Result<String> {
    let root: String = sqlx::query_scalar(
        "WITH RECURSIVE ancestors AS ( \
             SELECT id, parent_id FROM series WHERE id = $1 \
             UNION ALL \
             SELECT s.id, s.parent_id FROM series s JOIN ancestors a ON s.id = a.parent_id \
         ) SELECT id FROM ancestors WHERE parent_id IS NULL",
    )
    .bind(series_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| Error::NotFound {
        entity: "series",
        id: series_id.to_string(),
    })?;
    Ok(root)
}

/// Build the full tree from a root series, recursively loading children and articles.
pub async fn get_series_tree(pool: &PgPool, root_id: &str) -> crate::Result<SeriesTreeNode> {
    // Fetch all series in the tree in one query using recursive CTE
    let all_series = sqlx::query_as::<_, SeriesRow>(
        "WITH RECURSIVE tree AS ( \
             SELECT id, title, description, parent_id, order_index, created_by, created_at \
             FROM series WHERE id = $1 \
             UNION ALL \
             SELECT s.id, s.title, s.description, s.tag_id, s.parent_id, s.order_index, s.created_by, s.created_at \
             FROM series s JOIN tree t ON s.parent_id = t.id \
         ) SELECT * FROM tree",
    )
    .bind(root_id)
    .fetch_all(pool)
    .await?;

    if all_series.is_empty() {
        return Err(Error::NotFound {
            entity: "series",
            id: root_id.to_string(),
        });
    }

    // Fetch all articles for all series in the tree in one query
    let series_ids: Vec<&str> = all_series.iter().map(|s| s.id.as_str()).collect();
    let all_articles = sqlx::query_as::<_, SeriesArticleRow>(
        "SELECT sa.series_id, sa.article_uri, a.title, COALESCE(a.description, '') AS description, \
                a.lang, sa.order_index \
         FROM series_articles sa JOIN articles a ON sa.article_uri = a.at_uri \
         WHERE sa.series_id = ANY($1) ORDER BY sa.order_index",
    )
    .bind(&series_ids)
    .fetch_all(pool)
    .await?;

    // Group articles by series_id
    let mut articles_map: std::collections::HashMap<String, Vec<SeriesArticleRow>> =
        std::collections::HashMap::new();
    for art in all_articles {
        articles_map.entry(art.series_id.clone()).or_default().push(art);
    }

    // Build tree recursively
    fn build_node(
        id: &str,
        series_map: &std::collections::HashMap<String, SeriesRow>,
        children_map: &std::collections::HashMap<String, Vec<String>>,
        articles_map: &mut std::collections::HashMap<String, Vec<SeriesArticleRow>>,
    ) -> SeriesTreeNode {
        let series = series_map[id].clone();
        let articles = articles_map.remove(id).unwrap_or_default();
        let mut children: Vec<SeriesTreeNode> = children_map
            .get(id)
            .map(|child_ids| {
                child_ids
                    .iter()
                    .map(|cid| build_node(cid, series_map, children_map, articles_map))
                    .collect()
            })
            .unwrap_or_default();
        children.sort_by_key(|c| c.series.order_index);
        SeriesTreeNode {
            series,
            articles,
            children,
        }
    }

    let mut series_map: std::collections::HashMap<String, SeriesRow> = std::collections::HashMap::new();
    let mut children_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for s in &all_series {
        series_map.insert(s.id.clone(), s.clone());
        if let Some(pid) = &s.parent_id {
            children_map.entry(pid.clone()).or_default().push(s.id.clone());
        }
    }

    Ok(build_node(root_id, &series_map, &children_map, &mut articles_map))
}

/// Reorder articles within a series.
pub async fn reorder_series_articles(
    pool: &PgPool,
    series_id: &str,
    article_uris: &[String],
) -> crate::Result<()> {
    for (i, uri) in article_uris.iter().enumerate() {
        sqlx::query(
            "UPDATE series_articles SET order_index = $1 WHERE series_id = $2 AND article_uri = $3",
        )
        .bind(i as i32)
        .bind(series_id)
        .bind(uri)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Reorder child series within a parent.
pub async fn reorder_children(
    pool: &PgPool,
    parent_id: &str,
    child_ids: &[String],
) -> crate::Result<()> {
    for (i, cid) in child_ids.iter().enumerate() {
        sqlx::query("UPDATE series SET order_index = $1 WHERE id = $2 AND parent_id = $3")
            .bind(i as i32)
            .bind(cid)
            .bind(parent_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn get_series_context(
    pool: &PgPool,
    article_uri: &str,
) -> crate::Result<Vec<SeriesContextItem>> {
    let series_ids: Vec<String> = sqlx::query_scalar(
        "SELECT series_id FROM series_articles WHERE article_uri = $1",
    )
    .bind(article_uri)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for sid in series_ids {
        let series_title = sqlx::query_scalar::<_, String>(
            "SELECT title FROM series WHERE id = $1",
        )
        .bind(&sid)
        .fetch_optional(pool)
        .await?
        .unwrap_or_default();

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM series_articles WHERE series_id = $1",
        )
        .bind(&sid)
        .fetch_one(pool)
        .await?;

        // Prev: articles that are direct prerequisites of this one
        let prev = sqlx::query_as::<_, SeriesNavItem>(
            "SELECT sp.prereq_article_uri AS article_uri, a.title \
             FROM series_article_prereqs sp \
             JOIN articles a ON a.at_uri = sp.prereq_article_uri \
             WHERE sp.series_id = $1 AND sp.article_uri = $2",
        )
        .bind(&sid)
        .bind(article_uri)
        .fetch_all(pool)
        .await?;

        // Next: articles that require this one as a prerequisite
        let next = sqlx::query_as::<_, SeriesNavItem>(
            "SELECT sp.article_uri, a.title \
             FROM series_article_prereqs sp \
             JOIN articles a ON a.at_uri = sp.article_uri \
             WHERE sp.series_id = $1 AND sp.prereq_article_uri = $2",
        )
        .bind(&sid)
        .bind(article_uri)
        .fetch_all(pool)
        .await?;

        result.push(SeriesContextItem {
            series_id: sid,
            series_title,
            total,
            prev,
            next,
        });
    }

    Ok(result)
}
