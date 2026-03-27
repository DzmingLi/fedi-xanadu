use sqlx::PgPool;

use crate::models::*;

/// The base SELECT used for all article queries. Includes vote_score and
/// bookmark_count as correlated sub-selects, plus author_handle from profiles.
pub const ARTICLE_SELECT: &str = "\
    SELECT a.at_uri, a.did, p.handle AS author_handle, a.title, a.description, \
    a.content_hash, a.content_format, a.lang, a.translation_group, a.license, a.prereq_threshold, \
    COALESCE((SELECT SUM(value) FROM votes WHERE target_uri = a.at_uri), 0) AS vote_score, \
    COALESCE((SELECT COUNT(*) FROM user_bookmarks WHERE article_uri = a.at_uri), 0) AS bookmark_count, \
    a.created_at, a.updated_at \
    FROM articles a LEFT JOIN profiles p ON a.did = p.did";

// ---- Row types local to this service ----

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ContentTeachRow {
    pub content_uri: String,
    pub tag_id: String,
    pub tag_name: String,
    pub tag_names: sqlx::types::Json<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ContentPrereqBulkRow {
    pub content_uri: String,
    pub tag_id: String,
    pub prereq_type: String,
    pub tag_name: String,
    pub tag_names: sqlx::types::Json<std::collections::HashMap<String, String>>,
}

// ---- Service functions ----

/// Fetch recent articles with pagination.
pub async fn list_articles(pool: &PgPool, limit: i64, offset: i64) -> crate::Result<Vec<Article>> {
    let articles = sqlx::query_as::<_, Article>(&format!(
        "{ARTICLE_SELECT} ORDER BY a.created_at DESC LIMIT $1 OFFSET $2"
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(articles)
}

/// Get a single article by its AT URI.
pub async fn get_article(pool: &PgPool, uri: &str) -> crate::Result<Article> {
    sqlx::query_as::<_, Article>(&format!("{ARTICLE_SELECT} WHERE a.at_uri = $1"))
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound {
            entity: "article",
            id: uri.to_string(),
        })
}

/// Create a new article with its tags and prereqs inside a transaction.
pub async fn create_article(
    pool: &PgPool,
    did: &str,
    at_uri: &str,
    input: &CreateArticle,
    content_hash: &str,
    translation_group: Option<String>,
) -> crate::Result<Article> {
    let lang = input.lang.as_deref().unwrap_or("zh");
    let license = input.license.as_deref().unwrap_or("CC-BY-NC-SA-4.0");

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO articles (at_uri, did, title, description, content_hash, content_format, lang, translation_group, license, prereq_threshold) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0.8)",
    )
    .bind(at_uri)
    .bind(did)
    .bind(&input.title)
    .bind(input.description.as_deref().unwrap_or(""))
    .bind(content_hash)
    .bind(&input.content_format)
    .bind(lang)
    .bind(&translation_group)
    .bind(license)
    .execute(&mut *tx)
    .await?;

    for tag_id in &input.tags {
        sqlx::query(
            "INSERT INTO tags (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
        )
        .bind(tag_id)
        .bind(tag_id)
        .bind(did)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(at_uri)
        .bind(tag_id)
        .execute(&mut *tx)
        .await?;
    }

    for prereq in &input.prereqs {
        sqlx::query(
            "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(at_uri)
        .bind(&prereq.tag_id)
        .bind(prereq.prereq_type.as_str())
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // Return the newly-created article with all computed columns.
    let article = sqlx::query_as::<_, Article>(&format!("{ARTICLE_SELECT} WHERE a.at_uri = $1"))
        .bind(at_uri)
        .fetch_one(pool)
        .await?;

    Ok(article)
}

/// Get prereqs for a single article.
pub async fn get_article_prereqs(
    pool: &PgPool,
    uri: &str,
) -> crate::Result<Vec<ArticlePrereqRow>> {
    let rows = sqlx::query_as::<_, ArticlePrereqRow>(
        "SELECT cp.tag_id, cp.prereq_type, t.name as tag_name, t.names as tag_names \
         FROM content_prereqs cp \
         JOIN tags t ON t.id = cp.tag_id \
         WHERE cp.content_uri = $1",
    )
    .bind(uri)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get all forks whose source is the given article URI.
pub async fn get_article_forks(pool: &PgPool, uri: &str) -> crate::Result<Vec<ForkWithTitle>> {
    let rows = sqlx::query_as::<_, ForkWithTitle>(
        "SELECT f.fork_uri, f.forked_uri, f.vote_score, a.title, a.did, p.handle AS author_handle \
         FROM forks f \
         JOIN articles a ON a.at_uri = f.forked_uri \
         LEFT JOIN profiles p ON a.did = p.did \
         WHERE f.source_uri = $1 \
         ORDER BY f.vote_score DESC",
    )
    .bind(uri)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Bulk-fetch all article-tag mappings (with safety limit).
pub async fn get_all_article_teaches(pool: &PgPool, limit: i64) -> crate::Result<Vec<ContentTeachRow>> {
    let rows = sqlx::query_as::<_, ContentTeachRow>(
        "SELECT ct.content_uri, ct.tag_id, t.name as tag_name, t.names as tag_names \
         FROM content_teaches ct \
         JOIN tags t ON t.id = ct.tag_id \
         JOIN content c ON c.uri = ct.content_uri AND c.content_type = 'article' \
         ORDER BY ct.content_uri LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Bulk-fetch all article prereqs (with safety limit).
pub async fn get_all_article_prereqs(pool: &PgPool, limit: i64) -> crate::Result<Vec<ContentPrereqBulkRow>> {
    let rows = sqlx::query_as::<_, ContentPrereqBulkRow>(
        "SELECT cp.content_uri, cp.tag_id, cp.prereq_type, t.name as tag_name, t.names as tag_names \
         FROM content_prereqs cp \
         JOIN tags t ON t.id = cp.tag_id \
         JOIN content c ON c.uri = cp.content_uri AND c.content_type = 'article' \
         ORDER BY cp.content_uri LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Find articles tagged with the given tag or any of its descendant tags.
pub async fn get_articles_by_tag(pool: &PgPool, tag_id: &str, limit: i64) -> crate::Result<Vec<Article>> {
    // Collect the tag itself plus all descendant tags via recursive CTE.
    let descendant_tags: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE descendants(tag) AS ( \
           SELECT $1::TEXT \
           UNION \
           SELECT e.child_tag FROM skill_tree_edges e JOIN descendants d ON e.parent_tag = d.tag \
         ) \
         SELECT tag FROM descendants",
    )
    .bind(tag_id)
    .fetch_all(pool)
    .await?;

    if descendant_tags.is_empty() {
        return Ok(vec![]);
    }

    // Use ANY($1) with PostgreSQL array parameter
    let sql = format!(
        "{ARTICLE_SELECT} WHERE a.at_uri IN (\
            SELECT ct.content_uri FROM content_teaches ct WHERE ct.tag_id = ANY($1)\
         ) \
         ORDER BY a.created_at DESC LIMIT $2"
    );

    let articles = sqlx::query_as::<_, Article>(&sql)
        .bind(&descendant_tags)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(articles)
}

/// Get all articles authored by a given DID.
pub async fn get_articles_by_did(pool: &PgPool, did: &str, limit: i64) -> crate::Result<Vec<Article>> {
    let articles = sqlx::query_as::<_, Article>(&format!(
        "{ARTICLE_SELECT} WHERE a.did = $1 ORDER BY a.created_at DESC LIMIT $2"
    ))
    .bind(did)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(articles)
}

/// Get other translations in the same translation group (excluding the given URI).
pub async fn get_translations(pool: &PgPool, uri: &str) -> crate::Result<Vec<Article>> {
    let group: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(translation_group, at_uri) FROM articles WHERE at_uri = $1",
    )
    .bind(uri)
    .fetch_optional(pool)
    .await?;

    let Some(group) = group else {
        return Ok(vec![]);
    };

    let articles = sqlx::query_as::<_, Article>(&format!(
        "{ARTICLE_SELECT} WHERE a.translation_group = $1 AND a.at_uri != $2 ORDER BY a.lang"
    ))
    .bind(&group)
    .bind(uri)
    .fetch_all(pool)
    .await?;
    Ok(articles)
}

/// Resolve (or create) the translation group for a source article.
///
/// Returns the translation_group value. If the source article has no group yet,
/// its own at_uri is used and persisted.
pub async fn resolve_translation_group(
    pool: &PgPool,
    source_uri: &str,
) -> crate::Result<String> {
    let group: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(translation_group, at_uri) FROM articles WHERE at_uri = $1",
    )
    .bind(source_uri)
    .fetch_optional(pool)
    .await?;

    let g = group.ok_or_else(|| crate::Error::NotFound {
        entity: "article",
        id: source_uri.to_string(),
    })?;

    // Ensure the source article itself has the group persisted.
    sqlx::query(
        "UPDATE articles SET translation_group = $1 WHERE at_uri = $2 AND translation_group IS NULL",
    )
    .bind(&g)
    .bind(source_uri)
    .execute(pool)
    .await?;

    Ok(g)
}

/// Create a fork record: insert the forked article, copy tags/prereqs, and
/// insert the forks row. Returns the newly created article.
pub async fn create_fork_record(
    pool: &PgPool,
    fork_uri: &str,
    source_uri: &str,
    forked_uri: &str,
    did: &str,
    source: &Article,
) -> crate::Result<Article> {
    sqlx::query(
        "INSERT INTO articles (at_uri, did, title, content_hash, content_format, prereq_threshold) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(forked_uri)
    .bind(did)
    .bind(format!("Fork: {}", source.title))
    .bind(&source.content_hash)
    .bind(&source.content_format)
    .bind(source.prereq_threshold)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) \
         SELECT $1, tag_id, prereq_type FROM content_prereqs WHERE content_uri = $2 \
         ON CONFLICT DO NOTHING",
    )
    .bind(forked_uri)
    .bind(source_uri)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO content_teaches (content_uri, tag_id) \
         SELECT $1, tag_id FROM content_teaches WHERE content_uri = $2 \
         ON CONFLICT DO NOTHING",
    )
    .bind(forked_uri)
    .bind(source_uri)
    .execute(pool)
    .await?;

    sqlx::query("INSERT INTO forks (fork_uri, source_uri, forked_uri) VALUES ($1, $2, $3)")
        .bind(fork_uri)
        .bind(source_uri)
        .bind(forked_uri)
        .execute(pool)
        .await?;

    let article = sqlx::query_as::<_, Article>(&format!("{ARTICLE_SELECT} WHERE a.at_uri = $1"))
        .bind(forked_uri)
        .fetch_one(pool)
        .await?;

    Ok(article)
}

/// Update only the title of an article.
pub async fn update_article_title(pool: &PgPool, uri: &str, title: &str) -> crate::Result<()> {
    sqlx::query("UPDATE articles SET title = $1, updated_at = NOW() WHERE at_uri = $2")
        .bind(title)
        .bind(uri)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update only the description of an article.
pub async fn update_article_description(
    pool: &PgPool,
    uri: &str,
    desc: &str,
) -> crate::Result<()> {
    sqlx::query("UPDATE articles SET description = $1, updated_at = NOW() WHERE at_uri = $2")
        .bind(desc)
        .bind(uri)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update only the content_hash of an article.
pub async fn update_article_content_hash(
    pool: &PgPool,
    uri: &str,
    hash: &str,
) -> crate::Result<()> {
    sqlx::query("UPDATE articles SET content_hash = $1, updated_at = NOW() WHERE at_uri = $2")
        .bind(hash)
        .bind(uri)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete an article. Most associated data is cleaned up by ON DELETE CASCADE.
/// Only `votes` lacks a FK to articles, so we clean it up manually.
pub async fn delete_article(pool: &PgPool, uri: &str) -> crate::Result<()> {
    let mut tx = pool.begin().await?;

    // votes.target_uri is not a FK, clean up manually
    sqlx::query("DELETE FROM votes WHERE target_uri = $1")
        .bind(uri)
        .execute(&mut *tx)
        .await?;

    // CASCADE handles: content_teaches, content_prereqs (via content trigger), forks, user_bookmarks,
    // series_articles, comments (+ comment_votes via comments CASCADE)
    sqlx::query("DELETE FROM articles WHERE at_uri = $1")
        .bind(uri)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Return the content_format of an article.
pub async fn get_content_format(pool: &PgPool, uri: &str) -> crate::Result<String> {
    sqlx::query_scalar::<_, String>("SELECT content_format FROM articles WHERE at_uri = $1")
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound {
            entity: "article",
            id: uri.to_string(),
        })
}

/// Batch-fetch articles by a list of URIs, preserving the input order.
pub async fn get_articles_by_uris(pool: &PgPool, uris: &[String]) -> crate::Result<Vec<Article>> {
    if uris.is_empty() {
        return Ok(vec![]);
    }
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{ARTICLE_SELECT} WHERE a.at_uri = ANY($1)"
    ))
    .bind(uris)
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for a in rows {
        map.insert(a.at_uri.clone(), a);
    }
    Ok(uris.iter().filter_map(|u| map.get(u).cloned()).collect())
}

/// Return the DID of the article owner, or a NotFound error.
pub async fn get_article_owner(pool: &PgPool, uri: &str) -> crate::Result<String> {
    sqlx::query_scalar::<_, String>("SELECT did FROM articles WHERE at_uri = $1")
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound {
            entity: "article",
            id: uri.to_string(),
        })
}

/// Auto-bookmark an article into the user's folder.
/// Silently ignores conflicts (already bookmarked).
pub async fn auto_bookmark(pool: &PgPool, did: &str, uri: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO user_bookmarks (did, article_uri, folder_path) VALUES ($1, $2, '我的文章') ON CONFLICT DO NOTHING",
    )
    .bind(did)
    .bind(uri)
    .execute(pool)
    .await?;
    Ok(())
}
