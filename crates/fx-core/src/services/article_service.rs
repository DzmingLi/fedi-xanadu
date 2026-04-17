use sqlx::PgPool;

use crate::content::ContentKind;
use crate::models::*;
use crate::region::{InstanceMode, visibility_filter};

/// Base SELECT for article queries (no WHERE clause).
const ARTICLE_BASE: &str = "\
    SELECT a.at_uri, a.did, p.handle AS author_handle, p.display_name AS author_display_name, p.avatar_url AS author_avatar, COALESCE(p.reputation, 0) AS author_reputation, \
    a.kind, a.title, a.description, a.description_html, a.cover_url, \
    a.content_hash, a.content_format, a.lang, a.translation_group, a.license, a.prereq_threshold, \
    a.question_uri, a.answer_count, a.restricted, a.category, a.book_id, a.edition_id, \
    COALESCE(v.vote_score, 0) AS vote_score, \
    COALESCE(b.bookmark_count, 0) AS bookmark_count, \
    COALESCE(cm.comment_count, 0) AS comment_count, \
    COALESCE(fk.fork_count, 0) AS fork_count, \
    a.created_at, a.updated_at \
    FROM articles a \
    LEFT JOIN profiles p ON a.did = p.did \
    LEFT JOIN (SELECT target_uri, SUM(value) AS vote_score FROM votes GROUP BY target_uri) v ON v.target_uri = a.at_uri \
    LEFT JOIN (SELECT article_uri, COUNT(*) AS bookmark_count FROM user_bookmarks GROUP BY article_uri) b ON b.article_uri = a.at_uri \
    LEFT JOIN (SELECT content_uri, COUNT(*) AS comment_count FROM comments GROUP BY content_uri) cm ON cm.content_uri = a.at_uri \
    LEFT JOIN (SELECT source_uri, COUNT(*) AS fork_count FROM forks GROUP BY source_uri) fk ON fk.source_uri = a.at_uri";

/// Build article SELECT with instance-appropriate visibility filter.
fn visible(mode: InstanceMode) -> String {
    format!("{ARTICLE_BASE} WHERE {}", visibility_filter(mode))
}

// ---- Row types local to this service ----

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ContentTeachRow {
    pub content_uri: String,
    pub tag_id: String,
    pub tag_name: String,
    #[ts(type = "Record<string, string>")]

    pub tag_names: sqlx::types::Json<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ContentPrereqBulkRow {
    pub content_uri: String,
    pub tag_id: String,
    pub prereq_type: String,
    pub tag_name: String,
    #[ts(type = "Record<string, string>")]

    pub tag_names: sqlx::types::Json<std::collections::HashMap<String, String>>,
}

// ---- Queries (respect visibility) ----

pub async fn list_articles(pool: &PgPool, mode: InstanceMode, limit: i64, offset: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'article' ORDER BY a.created_at DESC LIMIT $1 OFFSET $2", visible(mode)
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_questions_by_book(pool: &PgPool, mode: InstanceMode, book_id: &str, limit: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'question' AND a.book_id = $1 ORDER BY vote_score DESC, a.created_at DESC LIMIT $2", visible(mode)
    ))
    .bind(book_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Articles in the user's "Following" feed — authored by someone they follow,
/// OR cross-posted to a publication they follow. Deduplicated by at_uri.
pub async fn list_following_feed(pool: &PgPool, mode: InstanceMode, did: &str, limit: i64, offset: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND ( \
            a.did IN (SELECT follows_did FROM user_follows WHERE did = $1) \
            OR a.at_uri IN ( \
                SELECT pc.content_uri FROM publication_content pc \
                JOIN publication_followers pf ON pf.publication_id = pc.publication_id \
                WHERE pc.content_kind = 'article' AND pf.did = $1 \
            ) \
         ) \
         ORDER BY a.created_at DESC LIMIT $2 OFFSET $3", visible(mode)
    ))
    .bind(did)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_thoughts(pool: &PgPool, mode: InstanceMode, limit: i64, offset: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'thought' ORDER BY a.created_at DESC LIMIT $1 OFFSET $2", visible(mode)
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_questions(pool: &PgPool, mode: InstanceMode, limit: i64, offset: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'question' ORDER BY a.created_at DESC LIMIT $1 OFFSET $2", visible(mode)
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_answers(pool: &PgPool, mode: InstanceMode, question_uri: &str, limit: i64, offset: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'answer' AND a.question_uri = $1 ORDER BY vote_score DESC, a.created_at ASC LIMIT $2 OFFSET $3", visible(mode)
    ))
    .bind(question_uri)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_article(pool: &PgPool, mode: InstanceMode, uri: &str) -> crate::Result<Article> {
    sqlx::query_as::<_, Article>(&format!("{} AND a.at_uri = $1", visible(mode)))
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() })
}

pub async fn get_questions_by_tag(pool: &PgPool, mode: InstanceMode, tag_id: &str, limit: i64) -> crate::Result<Vec<Article>> {
    let descendant_tags: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE descendants(tag) AS ( \
           SELECT $1::TEXT UNION \
           SELECT e.child_tag FROM skill_tree_edges e JOIN descendants d ON e.parent_tag = d.tag \
         ) SELECT tag FROM descendants",
    )
    .bind(tag_id)
    .fetch_all(pool)
    .await?;

    if descendant_tags.is_empty() {
        return Ok(vec![]);
    }

    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'question' AND a.at_uri IN (\
            SELECT ct.content_uri FROM content_teaches ct WHERE ct.tag_id = ANY($1)\
         ) ORDER BY a.created_at DESC LIMIT $2",
        visible(mode)
    ))
    .bind(&descendant_tags)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Find questions that share teaches-tags with the given question, excluding itself.
pub async fn get_related_questions(pool: &PgPool, mode: InstanceMode, uri: &str, limit: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'question' AND a.at_uri != $1 AND a.at_uri IN (\
            SELECT ct2.content_uri FROM content_teaches ct1 \
            JOIN content_teaches ct2 ON ct1.tag_id = ct2.tag_id \
            WHERE ct1.content_uri = $1 AND ct2.content_uri != $1\
         ) ORDER BY vote_score DESC, a.created_at DESC LIMIT $2",
        visible(mode)
    ))
    .bind(uri)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_articles_by_tag(pool: &PgPool, mode: InstanceMode, tag_id: &str, limit: i64) -> crate::Result<Vec<Article>> {
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

    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.at_uri IN (\
            SELECT ct.content_uri FROM content_teaches ct WHERE ct.tag_id = ANY($1)\
         ) ORDER BY a.created_at DESC LIMIT $2",
        visible(mode)
    ))
    .bind(&descendant_tags)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_articles_by_did(pool: &PgPool, mode: InstanceMode, did: &str, limit: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'article' AND a.did = $1 ORDER BY a.created_at DESC LIMIT $2", visible(mode)
    ))
    .bind(did)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_questions_by_did(pool: &PgPool, mode: InstanceMode, did: &str, limit: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'question' AND a.did = $1 ORDER BY a.created_at DESC LIMIT $2", visible(mode)
    ))
    .bind(did)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_answers_by_did(pool: &PgPool, mode: InstanceMode, did: &str, limit: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'answer' AND a.did = $1 ORDER BY a.created_at DESC LIMIT $2", visible(mode)
    ))
    .bind(did)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_translations(pool: &PgPool, mode: InstanceMode, uri: &str) -> crate::Result<Vec<Article>> {
    let group: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(translation_group, at_uri) FROM articles WHERE at_uri = $1",
    )
    .bind(uri)
    .fetch_optional(pool)
    .await?;

    let Some(group) = group else { return Ok(vec![]); };

    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.translation_group = $1 AND a.at_uri != $2 ORDER BY a.lang", visible(mode)
    ))
    .bind(&group)
    .bind(uri)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_articles_by_uris(pool: &PgPool, mode: InstanceMode, uris: &[String]) -> crate::Result<Vec<Article>> {
    if uris.is_empty() {
        return Ok(vec![]);
    }
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.at_uri = ANY($1)", visible(mode)
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

// ---- Queries (bypass visibility — admin / internal) ----

pub async fn get_article_any_visibility(pool: &PgPool, uri: &str) -> crate::Result<Article> {
    sqlx::query_as::<_, Article>(&format!("{ARTICLE_BASE} WHERE a.at_uri = $1"))
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() })
}

pub async fn get_article_owner(pool: &PgPool, uri: &str) -> crate::Result<String> {
    sqlx::query_scalar::<_, String>("SELECT did FROM articles WHERE at_uri = $1")
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() })
}

pub async fn get_content_format(pool: &PgPool, uri: &str) -> crate::Result<String> {
    sqlx::query_scalar::<_, String>("SELECT content_format::text FROM articles WHERE at_uri = $1")
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() })
}

pub async fn get_article_prereqs(pool: &PgPool, uri: &str) -> crate::Result<Vec<ArticlePrereqRow>> {
    let rows = sqlx::query_as::<_, ArticlePrereqRow>(
        "SELECT cp.tag_id, cp.prereq_type, t.name as tag_name, t.names as tag_names \
         FROM content_prereqs cp JOIN tags t ON t.id = cp.tag_id \
         WHERE cp.content_uri = $1",
    )
    .bind(uri)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// If this article is a fork, returns the source article's URI.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ForkSourceInfo {
    pub source_uri: String,
    pub title: String,
    pub license: String,
    pub lang: String,
    pub did: String,
    pub author_handle: Option<String>,
    pub author_display_name: Option<String>,
    pub author_avatar: Option<String>,
}

/// Full metadata about a forked article's origin — title, license, and the
/// source author's public profile. The Article detail page uses this to
/// render an attribution banner for CC-BY-* derivative works.
pub async fn get_fork_source(pool: &PgPool, forked_uri: &str) -> crate::Result<Option<ForkSourceInfo>> {
    let row = sqlx::query_as::<_, ForkSourceInfo>(
        "SELECT f.source_uri, a.title, a.license, a.lang, a.did, \
                p.handle AS author_handle, p.display_name AS author_display_name, \
                p.avatar_url AS author_avatar \
         FROM forks f \
         JOIN articles a ON a.at_uri = f.source_uri \
         LEFT JOIN profiles p ON p.did = a.did \
         WHERE f.forked_uri = $1 \
         LIMIT 1",
    )
    .bind(forked_uri)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_article_forks(pool: &PgPool, uri: &str) -> crate::Result<Vec<ForkWithTitle>> {
    let rows = sqlx::query_as::<_, ForkWithTitle>(
        "SELECT f.fork_uri, f.forked_uri, f.vote_score, a.title, a.did, p.handle AS author_handle \
         FROM forks f JOIN articles a ON a.at_uri = f.forked_uri \
         LEFT JOIN profiles p ON a.did = p.did \
         WHERE f.source_uri = $1 ORDER BY f.vote_score DESC",
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

// ---- Mutations ----

/// Create a new article/question/answer with tags and prereqs.
/// `resolved_description` is the final description source (may be auto-extracted
/// from content, see `DescriptionInput` in the server layer); `description_html`
/// is the inline-rendered HTML cache for list views.
pub async fn create_article(
    pool: &PgPool,
    did: &str,
    at_uri: &str,
    input: &CreateArticle,
    content_hash: &str,
    translation_group: Option<String>,
    visibility: &str,
    kind: ContentKind,
    question_uri: Option<&str>,
    resolved_description: &str,
    description_html: &str,
) -> crate::Result<Article> {
    let lang = input.lang.as_deref().unwrap_or("zh");
    let restricted = input.restricted.unwrap_or(false);
    let license = if restricted { "All-Rights-Reserved" } else {
        input.license.as_deref().unwrap_or("CC-BY-SA-4.0")
    };

    let mut tx = pool.begin().await?;

    let category = input.category.as_deref().unwrap_or("general");

    sqlx::query(
        "INSERT INTO articles (at_uri, did, title, description, description_html, content_hash, content_format, lang, translation_group, license, prereq_threshold, visibility, kind, question_uri, restricted, category, book_id, edition_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 0.8, $11, $12, $13, $14, $15, $16, $17)",
    )
    .bind(at_uri)
    .bind(did)
    .bind(&input.title)
    .bind(resolved_description)
    .bind(description_html)
    .bind(content_hash)
    .bind(input.content_format)
    .bind(lang)
    .bind(&translation_group)
    .bind(license)
    .bind(visibility)
    .bind(kind)
    .bind(question_uri)
    .bind(restricted)
    .bind(category)
    .bind(input.review_book_id())
    .bind(input.review_edition_id())
    .execute(&mut *tx)
    .await?;

    for tag_id in &input.tags {
        sqlx::query(
            "INSERT INTO tags (id, name, created_by) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
        )
        .bind(tag_id).bind(tag_id).bind(did)
        .execute(&mut *tx).await?;

        sqlx::query(
            "INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(at_uri).bind(tag_id)
        .execute(&mut *tx).await?;
    }

    for prereq in &input.prereqs {
        sqlx::query(
            "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(at_uri).bind(&prereq.tag_id).bind(prereq.prereq_type.as_str())
        .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    let article = sqlx::query_as::<_, Article>(&format!("{ARTICLE_BASE} WHERE a.at_uri = $1"))
        .bind(at_uri)
        .fetch_one(pool)
        .await?;
    Ok(article)
}

/// Create a fork record: insert the forked article, copy tags/prereqs,
/// and insert the forks row.
pub async fn create_fork_record(
    pool: &PgPool,
    fork_uri: &str,
    source_uri: &str,
    forked_uri: &str,
    did: &str,
    source: &Article,
    visibility: &str,
) -> crate::Result<Article> {
    sqlx::query(
        "INSERT INTO articles (at_uri, did, title, content_hash, content_format, prereq_threshold, visibility) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(forked_uri).bind(did)
    .bind(format!("Fork: {}", source.title))
    .bind(&source.content_hash).bind(source.content_format)
    .bind(source.prereq_threshold).bind(visibility)
    .execute(pool).await?;

    sqlx::query(
        "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) \
         SELECT $1, tag_id, prereq_type FROM content_prereqs WHERE content_uri = $2 \
         ON CONFLICT DO NOTHING",
    )
    .bind(forked_uri).bind(source_uri)
    .execute(pool).await?;

    sqlx::query(
        "INSERT INTO content_teaches (content_uri, tag_id) \
         SELECT $1, tag_id FROM content_teaches WHERE content_uri = $2 \
         ON CONFLICT DO NOTHING",
    )
    .bind(forked_uri).bind(source_uri)
    .execute(pool).await?;

    sqlx::query("INSERT INTO forks (fork_uri, source_uri, forked_uri) VALUES ($1, $2, $3)")
        .bind(fork_uri).bind(source_uri).bind(forked_uri)
        .execute(pool).await?;

    let article = sqlx::query_as::<_, Article>(&format!("{ARTICLE_BASE} WHERE a.at_uri = $1"))
        .bind(forked_uri)
        .fetch_one(pool)
        .await?;
    Ok(article)
}

pub async fn update_article_title(pool: &PgPool, uri: &str, title: &str) -> crate::Result<()> {
    sqlx::query("UPDATE articles SET title = $1, updated_at = NOW() WHERE at_uri = $2")
        .bind(title).bind(uri).execute(pool).await?;
    Ok(())
}

pub async fn update_article_description(
    pool: &PgPool, uri: &str, desc: &str, desc_html: &str,
) -> crate::Result<()> {
    sqlx::query(
        "UPDATE articles SET description = $1, description_html = $2, updated_at = NOW() WHERE at_uri = $3",
    )
    .bind(desc).bind(desc_html).bind(uri).execute(pool).await?;
    Ok(())
}

pub async fn update_article_content_hash(pool: &PgPool, uri: &str, hash: &str) -> crate::Result<()> {
    sqlx::query("UPDATE articles SET content_hash = $1, updated_at = NOW() WHERE at_uri = $2")
        .bind(hash).bind(uri).execute(pool).await?;
    Ok(())
}

/// Unified visibility mutation. Handles `removed` specially (sets `removed_at`).
/// Transitioning away from `removed` clears `removed_at` / `remove_reason`.
pub async fn set_visibility(pool: &PgPool, uri: &str, visibility: &str, reason: Option<&str>) -> crate::Result<()> {
    let result = match visibility {
        "removed" => {
            sqlx::query(
                "UPDATE articles SET visibility = 'removed', removed_at = NOW(), remove_reason = $2 \
                 WHERE at_uri = $1 AND visibility != 'removed'",
            )
            .bind(uri).bind(reason)
            .execute(pool).await?
        }
        _ => {
            sqlx::query(
                "UPDATE articles SET visibility = $2, removed_at = NULL, remove_reason = NULL, updated_at = NOW() \
                 WHERE at_uri = $1",
            )
            .bind(uri).bind(visibility)
            .execute(pool).await?
        }
    };

    if result.rows_affected() == 0 {
        return Err(crate::Error::NotFound { entity: "article", id: uri.to_string() });
    }
    Ok(())
}

/// Update an article in-place (for batch re-publish). Keeps the same at_uri.
pub async fn update_article_batch(
    pool: &PgPool,
    uri: &str,
    input: &CreateArticle,
    content_hash: &str,
    resolved_description: &str,
    description_html: &str,
) -> crate::Result<Article> {
    let lang = input.lang.as_deref().unwrap_or("zh");
    let license = input.license.as_deref().unwrap_or("CC-BY-SA-4.0");
    let category = input.category.as_deref().unwrap_or("general");

    let mut tx = pool.begin().await?;

    sqlx::query(
        "UPDATE articles SET title = $2, description = $3, description_html = $9, content_hash = $4, \
         content_format = $5, lang = $6, license = $7, category = $8, updated_at = NOW() \
         WHERE at_uri = $1",
    )
    .bind(uri)
    .bind(&input.title)
    .bind(resolved_description)
    .bind(content_hash)
    .bind(input.content_format)
    .bind(lang)
    .bind(license)
    .bind(category)
    .bind(description_html)
    .execute(&mut *tx)
    .await?;

    // Replace tags
    sqlx::query("DELETE FROM content_teaches WHERE content_uri = $1")
        .bind(uri).execute(&mut *tx).await?;
    for tag_id in &input.tags {
        sqlx::query(
            "INSERT INTO tags (id, name, created_by) VALUES ($1, $2, (SELECT did FROM articles WHERE at_uri = $3)) ON CONFLICT (id) DO NOTHING",
        )
        .bind(tag_id).bind(tag_id).bind(uri)
        .execute(&mut *tx).await?;

        sqlx::query(
            "INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(uri).bind(tag_id)
        .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    let article = sqlx::query_as::<_, Article>(&format!("{ARTICLE_BASE} WHERE a.at_uri = $1"))
        .bind(uri)
        .fetch_one(pool)
        .await?;
    Ok(article)
}

/// Hard-delete an article. CASCADE handles most FKs; votes cleaned up manually.
pub async fn delete_article(pool: &PgPool, uri: &str) -> crate::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM votes WHERE target_uri = $1")
        .bind(uri).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM articles WHERE at_uri = $1")
        .bind(uri).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Hard-delete articles removed more than 30 days ago.
pub async fn cleanup_expired_removals(pool: &PgPool) -> crate::Result<u64> {
    let uris: Vec<String> = sqlx::query_scalar(
        "SELECT at_uri FROM articles WHERE visibility = 'removed' AND removed_at < NOW() - INTERVAL '30 days'",
    )
    .fetch_all(pool).await?;

    if uris.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM votes WHERE target_uri = ANY($1)")
        .bind(&uris).execute(&mut *tx).await?;
    let result = sqlx::query("DELETE FROM articles WHERE at_uri = ANY($1)")
        .bind(&uris).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(result.rows_affected())
}

/// Resolve (or create) the translation group for a source article.
pub async fn resolve_translation_group(pool: &PgPool, source_uri: &str) -> crate::Result<String> {
    let group: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(translation_group, at_uri) FROM articles WHERE at_uri = $1",
    )
    .bind(source_uri)
    .fetch_optional(pool)
    .await?;

    let g = group.ok_or_else(|| crate::Error::NotFound {
        entity: "article", id: source_uri.to_string(),
    })?;

    sqlx::query(
        "UPDATE articles SET translation_group = $1 WHERE at_uri = $2 AND translation_group IS NULL",
    )
    .bind(&g).bind(source_uri)
    .execute(pool).await?;

    Ok(g)
}

/// Merge question `from_uri` into `into_uri`: move all answers, log merge, delete old question.
pub async fn merge_questions(pool: &PgPool, from_uri: &str, into_uri: &str) -> crate::Result<u64> {
    let mut tx = pool.begin().await?;

    // Move answers
    let result = sqlx::query(
        "UPDATE articles SET question_uri = $2 WHERE question_uri = $1 AND kind = 'answer'",
    )
    .bind(from_uri).bind(into_uri)
    .execute(&mut *tx).await?;
    let moved = result.rows_affected();

    // Recount answer_count on both questions
    sqlx::query(
        "UPDATE articles SET answer_count = (SELECT COUNT(*) FROM articles WHERE question_uri = $1 AND kind = 'answer') WHERE at_uri = $1",
    )
    .bind(into_uri).execute(&mut *tx).await?;

    // Log merge
    sqlx::query(
        "INSERT INTO question_merges (from_uri, into_uri) VALUES ($1, $2) ON CONFLICT (from_uri) DO NOTHING",
    )
    .bind(from_uri).bind(into_uri)
    .execute(&mut *tx).await?;

    // Delete old question (CASCADE handles content_teaches, content_prereqs, etc.)
    delete_article_in_tx(&mut tx, from_uri).await?;

    tx.commit().await?;
    Ok(moved)
}

async fn delete_article_in_tx(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, uri: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM votes WHERE target_uri = $1")
        .bind(uri).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM articles WHERE at_uri = $1")
        .bind(uri).execute(&mut **tx).await?;
    Ok(())
}

/// Auto-bookmark an article into the user's folder.
pub async fn auto_bookmark(pool: &PgPool, did: &str, uri: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO user_bookmarks (did, article_uri, folder_path) VALUES ($1, $2, '我的文章') ON CONFLICT DO NOTHING",
    )
    .bind(did).bind(uri)
    .execute(pool).await?;
    Ok(())
}

// --- Access control (paywall) ---

/// Check if a viewer can access article content.
/// Returns true if unrestricted, or if viewer is owner or has a grant.
pub async fn check_content_access(pool: &PgPool, uri: &str, viewer_did: Option<&str>) -> crate::Result<bool> {
    let row: Option<(String, bool)> = sqlx::query_as(
        "SELECT did, restricted FROM articles WHERE at_uri = $1",
    )
    .bind(uri)
    .fetch_optional(pool)
    .await?;

    let Some((owner_did, restricted)) = row else {
        return Err(crate::Error::NotFound { entity: "article", id: uri.to_string() });
    };

    if !restricted {
        return Ok(true);
    }

    let Some(viewer) = viewer_did else {
        return Ok(false);
    };

    if viewer == owner_did {
        return Ok(true);
    }

    // Check per-article grant OR user-level membership
    let granted: bool = sqlx::query_scalar(
        "SELECT EXISTS(\
            SELECT 1 FROM article_access_grants WHERE article_uri = $1 AND grantee_did = $2 \
            UNION ALL \
            SELECT 1 FROM user_members WHERE author_did = $3 AND member_did = $2 \
        )",
    )
    .bind(uri).bind(viewer).bind(&owner_did)
    .fetch_one(pool)
    .await?;

    Ok(granted)
}

pub async fn set_restricted(pool: &PgPool, uri: &str, restricted: bool) -> crate::Result<()> {
    if restricted {
        sqlx::query("UPDATE articles SET restricted = TRUE, license = 'All-Rights-Reserved', updated_at = NOW() WHERE at_uri = $1")
            .bind(uri).execute(pool).await?;
    } else {
        sqlx::query("UPDATE articles SET restricted = FALSE, updated_at = NOW() WHERE at_uri = $1")
            .bind(uri).execute(pool).await?;
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct AccessGrant {
    pub article_uri: String,
    pub grantee_did: String,
    pub granted_at: chrono::DateTime<chrono::Utc>,
}

pub async fn grant_access(pool: &PgPool, uri: &str, grantee_did: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO article_access_grants (article_uri, grantee_did) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(uri).bind(grantee_did)
    .execute(pool).await?;
    Ok(())
}

pub async fn revoke_access(pool: &PgPool, uri: &str, grantee_did: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM article_access_grants WHERE article_uri = $1 AND grantee_did = $2")
        .bind(uri).bind(grantee_did).execute(pool).await?;
    Ok(())
}

pub async fn list_access_grants(pool: &PgPool, uri: &str) -> crate::Result<Vec<AccessGrant>> {
    let rows = sqlx::query_as::<_, AccessGrant>(
        "SELECT article_uri, grantee_did, granted_at FROM article_access_grants WHERE article_uri = $1 ORDER BY granted_at",
    )
    .bind(uri)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ---- Paper metadata ----

pub async fn upsert_paper_metadata(
    pool: &PgPool,
    article_uri: &str,
    input: &crate::models::CreatePaperMetadata,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO paper_metadata (article_uri, venue, venue_type, year, doi, arxiv_id, accepted) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (article_uri) DO UPDATE SET \
           venue = EXCLUDED.venue, venue_type = EXCLUDED.venue_type, \
           year = EXCLUDED.year, doi = EXCLUDED.doi, \
           arxiv_id = EXCLUDED.arxiv_id, accepted = EXCLUDED.accepted",
    )
    .bind(article_uri)
    .bind(&input.venue)
    .bind(&input.venue_type)
    .bind(input.year)
    .bind(&input.doi)
    .bind(&input.arxiv_id)
    .bind(input.accepted)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_experience_metadata(
    pool: &PgPool,
    article_uri: &str,
    input: &crate::models::CreateExperienceMetadata,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO experience_metadata (article_uri, kind, target, year, result) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (article_uri) DO UPDATE SET \
           kind = EXCLUDED.kind, target = EXCLUDED.target, \
           year = EXCLUDED.year, result = EXCLUDED.result",
    )
    .bind(article_uri)
    .bind(&input.kind)
    .bind(&input.target)
    .bind(input.year)
    .bind(&input.result)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_experience_metadata(
    pool: &PgPool,
    article_uri: &str,
) -> crate::Result<Option<crate::models::ExperienceMetadata>> {
    let row = sqlx::query_as::<_, crate::models::ExperienceMetadata>(
        "SELECT article_uri, kind, target, year, result \
         FROM experience_metadata WHERE article_uri = $1",
    )
    .bind(article_uri)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_paper_metadata(
    pool: &PgPool,
    article_uri: &str,
) -> crate::Result<Option<crate::models::PaperMetadata>> {
    let row = sqlx::query_as::<_, crate::models::PaperMetadata>(
        "SELECT article_uri, venue, venue_type, year, doi, arxiv_id, accepted \
         FROM paper_metadata WHERE article_uri = $1",
    )
    .bind(article_uri)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
