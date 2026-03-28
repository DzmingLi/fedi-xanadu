use sqlx::PgPool;

use crate::models::*;
use crate::region::{InstanceMode, visibility_filter};

/// Base SELECT for article queries (no WHERE clause).
const ARTICLE_BASE: &str = "\
    SELECT a.at_uri, a.did, p.handle AS author_handle, a.kind, a.title, a.description, \
    a.content_hash, a.content_format, a.lang, a.translation_group, a.license, a.prereq_threshold, \
    a.question_uri, a.answer_count, a.restricted, a.category, a.book_id, \
    COALESCE((SELECT SUM(value) FROM votes WHERE target_uri = a.at_uri), 0) AS vote_score, \
    COALESCE((SELECT COUNT(*) FROM user_bookmarks WHERE article_uri = a.at_uri), 0) AS bookmark_count, \
    a.created_at, a.updated_at \
    FROM articles a LEFT JOIN profiles p ON a.did = p.did";

/// Build article SELECT with instance-appropriate visibility filter.
fn visible(mode: InstanceMode) -> String {
    format!("{ARTICLE_BASE} WHERE {}", visibility_filter(mode))
}

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
    sqlx::query_scalar::<_, String>("SELECT content_format FROM articles WHERE at_uri = $1")
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
pub async fn create_article(
    pool: &PgPool,
    did: &str,
    at_uri: &str,
    input: &CreateArticle,
    content_hash: &str,
    translation_group: Option<String>,
    visibility: &str,
    kind: &str,
    question_uri: Option<&str>,
) -> crate::Result<Article> {
    let lang = input.lang.as_deref().unwrap_or("zh");
    let restricted = input.restricted.unwrap_or(false);
    let license = if restricted { "All-Rights-Reserved" } else {
        input.license.as_deref().unwrap_or("CC-BY-SA-4.0")
    };

    let mut tx = pool.begin().await?;

    let category = input.category.as_deref().unwrap_or("general");

    sqlx::query(
        "INSERT INTO articles (at_uri, did, title, description, content_hash, content_format, lang, translation_group, license, prereq_threshold, visibility, kind, question_uri, restricted, category, book_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0.8, $10, $11, $12, $13, $14, $15)",
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
    .bind(visibility)
    .bind(kind)
    .bind(question_uri)
    .bind(restricted)
    .bind(category)
    .bind(input.book_id.as_deref())
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
    .bind(&source.content_hash).bind(&source.content_format)
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

pub async fn update_article_description(pool: &PgPool, uri: &str, desc: &str) -> crate::Result<()> {
    sqlx::query("UPDATE articles SET description = $1, updated_at = NOW() WHERE at_uri = $2")
        .bind(desc).bind(uri).execute(pool).await?;
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

    let mut count = 0u64;
    for uri in &uris {
        delete_article(pool, uri).await?;
        count += 1;
    }
    Ok(count)
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

    let granted: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM article_access_grants WHERE article_uri = $1 AND grantee_did = $2)",
    )
    .bind(uri).bind(viewer)
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

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
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
