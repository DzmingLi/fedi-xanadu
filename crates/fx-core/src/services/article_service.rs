use sqlx::PgPool;

use crate::content::ContentKind;
use crate::models::*;
use crate::region::{InstanceMode, visibility_filter};

/// Base SELECT for article queries. Joins the **source-language**
/// localization (where `file_path == source_path`). Aggregate joins key by
/// the synthetic article URI (via the `article_uri()` SQL function) so
/// single-column URI tables (votes, comments) keep working. Public so other
/// services can compose on top of it.
/// Same shape as [`ARTICLE_BASE`] but **without** the
/// `l.file_path = a.source_path` constraint on the localization join, so
/// callers can target a specific non-source localization (e.g. a translation)
/// by further filtering on `l.at_uri` or `l.lang`. The resulting view still
/// has all of the aggregate joins keyed by the article (not the localization)
/// so vote/bookmark/comment counts remain article-wide.
pub const ARTICLE_BASE_ANY_LANG: &str = "\
    SELECT a.repo_uri, a.source_path, a.author_did, \
    p.handle AS author_handle, p.display_name AS author_display_name, \
    p.avatar_url AS author_avatar, COALESCE(p.reputation, 0) AS author_reputation, \
    a.kind, a.category, a.license, a.prereq_threshold, a.restricted, a.answer_count, \
    a.cover_url, a.cover_file, \
    a.question_repo_uri, a.question_source_path, \
    a.book_id, a.edition_id, a.course_id, a.book_chapter_id, a.course_session_id, \
    l.lang, l.at_uri, l.file_path, l.title, l.summary, l.summary_html, \
    l.content_hash, l.content_format, l.translator_did, \
    pm.venue AS paper_venue, pm.year AS paper_year, pm.accepted AS paper_accepted, \
    COALESCE(v.vote_score, 0) AS vote_score, \
    COALESCE(b.bookmark_count, 0) AS bookmark_count, \
    COALESCE(cm.comment_count, 0) AS comment_count, \
    a.created_at, a.updated_at \
    FROM articles a \
    JOIN article_localizations l \
        ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
    LEFT JOIN profiles p ON p.did = a.author_did \
    LEFT JOIN paper_metadata pm \
        ON pm.repo_uri = a.repo_uri AND pm.source_path = a.source_path \
    LEFT JOIN (SELECT target_uri, SUM(value) AS vote_score FROM votes GROUP BY target_uri) v \
        ON v.target_uri = article_uri(a.repo_uri, a.source_path) \
    LEFT JOIN (SELECT repo_uri, source_path, COUNT(*) AS bookmark_count FROM user_bookmarks GROUP BY repo_uri, source_path) b \
        ON b.repo_uri = a.repo_uri AND b.source_path = a.source_path \
    LEFT JOIN (SELECT content_uri, COUNT(*) AS comment_count FROM comments GROUP BY content_uri) cm \
        ON cm.content_uri = article_uri(a.repo_uri, a.source_path)";

/// Fetch any specific localization (source or translation) as an [`Article`],
/// looked up by its ATProto record URI. Unlike [`get_article`], which is
/// pinned to the source language, this returns the view of the lang that
/// actually owns the passed `at_uri`.
pub async fn get_article_any_lang(pool: &PgPool, uri: &str) -> crate::Result<Article> {
    let sql = format!("{ARTICLE_BASE_ANY_LANG} WHERE l.at_uri = $1");
    sqlx::query_as::<_, Article>(&sql)
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() })
}

/// Resolve an article by either a localization's `at_uri` (for federated
/// articles) or its synthetic URI (`nightboat://article/{repo_uri}/{source_path}`,
/// the only usable handle for Q&A whose localizations have `at_uri = NULL`).
/// Returns the source-lang view. Callers that need a specific language
/// should follow up with `get_article_any_lang` on the selected localization's
/// at_uri.
pub async fn resolve_article(pool: &PgPool, uri: &str) -> crate::Result<Article> {
    if let Some(rest) = uri.strip_prefix("nightboat://article/") {
        // Synthetic URI: split into (repo_uri, source_path) at the LAST '/'.
        // `repo_uri` itself can contain slashes (e.g. `at://did/lex/rkey`), so
        // we use rsplit_once to peel off the trailing `source_path`.
        let Some((repo_uri, source_path)) = rest.rsplit_once('/') else {
            return Err(crate::Error::NotFound { entity: "article", id: uri.to_string() });
        };
        let sql = format!("{ARTICLE_BASE} WHERE a.repo_uri = $1 AND a.source_path = $2");
        return sqlx::query_as::<_, Article>(&sql)
            .bind(repo_uri)
            .bind(source_path)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() });
    }
    get_article_any_visibility(pool, uri).await
}

pub const ARTICLE_BASE: &str = "\
    SELECT a.repo_uri, a.source_path, a.author_did, \
    p.handle AS author_handle, p.display_name AS author_display_name, \
    p.avatar_url AS author_avatar, COALESCE(p.reputation, 0) AS author_reputation, \
    a.kind, a.category, a.license, a.prereq_threshold, a.restricted, a.answer_count, \
    a.cover_url, a.cover_file, \
    a.question_repo_uri, a.question_source_path, \
    a.book_id, a.edition_id, a.course_id, a.book_chapter_id, a.course_session_id, \
    l.lang, l.at_uri, l.file_path, l.title, l.summary, l.summary_html, \
    l.content_hash, l.content_format, l.translator_did, \
    pm.venue AS paper_venue, pm.year AS paper_year, pm.accepted AS paper_accepted, \
    COALESCE(v.vote_score, 0) AS vote_score, \
    COALESCE(b.bookmark_count, 0) AS bookmark_count, \
    COALESCE(cm.comment_count, 0) AS comment_count, \
    a.created_at, a.updated_at \
    FROM articles a \
    JOIN article_localizations l \
        ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
       AND l.file_path = a.source_path \
    LEFT JOIN profiles p ON p.did = a.author_did \
    LEFT JOIN paper_metadata pm \
        ON pm.repo_uri = a.repo_uri AND pm.source_path = a.source_path \
    LEFT JOIN (SELECT target_uri, SUM(value) AS vote_score FROM votes GROUP BY target_uri) v \
        ON v.target_uri = article_uri(a.repo_uri, a.source_path) \
    LEFT JOIN (SELECT repo_uri, source_path, COUNT(*) AS bookmark_count FROM user_bookmarks GROUP BY repo_uri, source_path) b \
        ON b.repo_uri = a.repo_uri AND b.source_path = a.source_path \
    LEFT JOIN (SELECT content_uri, COUNT(*) AS comment_count FROM comments GROUP BY content_uri) cm \
        ON cm.content_uri = article_uri(a.repo_uri, a.source_path)";

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
            a.author_did IN (SELECT follows_did FROM user_follows WHERE did = $1) \
            OR l.at_uri IN ( \
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
        "{} AND a.kind = 'answer' \
         AND (a.question_repo_uri, a.question_source_path) IN \
             (SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1) \
         ORDER BY vote_score DESC, a.created_at ASC LIMIT $2 OFFSET $3", visible(mode)
    ))
    .bind(question_uri)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_article(pool: &PgPool, mode: InstanceMode, uri: &str) -> crate::Result<Article> {
    if let Some((series_id, source_path)) = super::series_service::parse_chapter_uri(uri) {
        let repo_uri = super::series_service::series_repo_uri(pool, &series_id).await?;
        return sqlx::query_as::<_, Article>(&format!(
            "{} AND a.repo_uri = $1 AND a.source_path = $2",
            visible(mode)
        ))
        .bind(&repo_uri)
        .bind(&source_path)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() });
    }
    sqlx::query_as::<_, Article>(&format!("{} AND l.at_uri = $1", visible(mode)))
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() })
}

pub async fn get_questions_by_tag(pool: &PgPool, mode: InstanceMode, tag_id: &str, limit: i64) -> crate::Result<Vec<Article>> {
    // Walk the skill-tree taxonomy down from `tag_id` through descendants.
    let descendant_tags: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE descendants(tag) AS ( \
           SELECT CAST($1 AS varchar) \
           UNION \
           SELECT e.child_tag FROM skill_tree_edges e JOIN descendants d ON e.parent_tag = d.tag \
         ) SELECT tag FROM descendants WHERE tag IS NOT NULL",
    )
    .bind(tag_id)
    .fetch_all(pool)
    .await?;

    if descendant_tags.is_empty() {
        return Ok(vec![]);
    }

    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'question' AND article_uri(a.repo_uri, a.source_path) IN (\
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
        "{} AND a.kind = 'question' \
         AND article_uri(a.repo_uri, a.source_path) != $1 \
         AND article_uri(a.repo_uri, a.source_path) IN (\
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
    // Walk the skill-tree taxonomy down from `tag_id` through descendants.
    // Edge tables carry tag_ids directly so the filter is straightforward.
    let tag_ids: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE descendants(tid) AS ( \
             SELECT CAST($1 AS varchar) \
             UNION \
             SELECT e.child_tag \
             FROM skill_tree_edges e \
             JOIN descendants d ON d.tid = e.parent_tag \
         ) SELECT tid FROM descendants WHERE tid IS NOT NULL",
    )
    .bind(tag_id)
    .fetch_all(pool)
    .await?;

    if tag_ids.is_empty() {
        return Ok(vec![]);
    }

    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND article_uri(a.repo_uri, a.source_path) IN (\
            SELECT ct.content_uri FROM content_teaches ct WHERE ct.tag_id = ANY($1)\
         ) ORDER BY a.created_at DESC LIMIT $2",
        visible(mode)
    ))
    .bind(&tag_ids)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Articles that flag this tag as "related" (mentioned but not taught).
/// Used on the tag page alongside the teaches list. Unlike
/// `get_articles_by_tag` this does not walk taxonomy descendants —
/// related is a specific-concept pointer, so we match only the tag
/// itself.
pub async fn get_articles_related_by_tag(pool: &PgPool, mode: InstanceMode, tag_id: &str, limit: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND article_uri(a.repo_uri, a.source_path) IN (\
            SELECT cr.content_uri FROM content_related cr WHERE cr.tag_id = $1\
         ) ORDER BY a.created_at DESC LIMIT $2",
        visible(mode)
    ))
    .bind(tag_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_articles_by_did(pool: &PgPool, mode: InstanceMode, did: &str, limit: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'article' AND a.author_did = $1 ORDER BY a.created_at DESC LIMIT $2", visible(mode)
    ))
    .bind(did)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_questions_by_did(pool: &PgPool, mode: InstanceMode, did: &str, limit: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'question' AND a.author_did = $1 ORDER BY a.created_at DESC LIMIT $2", visible(mode)
    ))
    .bind(did)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_answers_by_did(pool: &PgPool, mode: InstanceMode, did: &str, limit: i64) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'answer' AND a.author_did = $1 ORDER BY a.created_at DESC LIMIT $2", visible(mode)
    ))
    .bind(did)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Return every other-language version of the article identified by `uri`
/// (the passed `uri` itself is excluded). `uri` is a localization's at_uri;
/// we resolve to `(repo_uri, source_path)` and enumerate sibling localizations.
///
/// Unlike most `get_*` helpers this does not reuse `ARTICLE_BASE` — that
/// template pins the source-language localization; here we want EVERY other
/// localization, each as its own display row.
pub async fn get_translations(pool: &PgPool, mode: InstanceMode, uri: &str) -> crate::Result<Vec<Article>> {
    let sql = format!(
        "SELECT a.repo_uri, a.source_path, a.author_did, \
            p.handle AS author_handle, p.display_name AS author_display_name, \
            p.avatar_url AS author_avatar, COALESCE(p.reputation, 0) AS author_reputation, \
            a.kind, a.category, a.license, a.prereq_threshold, a.restricted, a.answer_count, \
            a.cover_url, a.cover_file, \
            a.question_repo_uri, a.question_source_path, \
            a.book_id, a.edition_id, a.course_id, a.book_chapter_id, a.course_session_id, \
            l.lang, l.at_uri, l.file_path, l.title, l.summary, l.summary_html, \
            l.content_hash, l.content_format, l.translator_did, \
            pm.venue AS paper_venue, pm.year AS paper_year, pm.accepted AS paper_accepted, \
            COALESCE(v.vote_score, 0) AS vote_score, \
            COALESCE(b.bookmark_count, 0) AS bookmark_count, \
            COALESCE(cm.comment_count, 0) AS comment_count, \
            a.created_at, a.updated_at \
         FROM articles a \
         JOIN article_localizations l \
             ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
         LEFT JOIN profiles p ON p.did = a.author_did \
         LEFT JOIN paper_metadata pm \
             ON pm.repo_uri = a.repo_uri AND pm.source_path = a.source_path \
         LEFT JOIN (SELECT target_uri, SUM(value) AS vote_score FROM votes GROUP BY target_uri) v \
             ON v.target_uri = article_uri(a.repo_uri, a.source_path) \
         LEFT JOIN (SELECT repo_uri, source_path, COUNT(*) AS bookmark_count FROM user_bookmarks GROUP BY repo_uri, source_path) b \
             ON b.repo_uri = a.repo_uri AND b.source_path = a.source_path \
         LEFT JOIN (SELECT content_uri, COUNT(*) AS comment_count FROM comments GROUP BY content_uri) cm \
             ON cm.content_uri = article_uri(a.repo_uri, a.source_path) \
         WHERE (a.repo_uri, a.source_path) IN ( \
             SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1 \
         ) AND l.at_uri IS DISTINCT FROM $1 AND {} \
         ORDER BY l.lang",
        visibility_filter(mode)
    );
    let rows = sqlx::query_as::<_, Article>(&sql)
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
        "{} AND l.at_uri = ANY($1)", visible(mode)
    ))
    .bind(uris)
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for a in rows {
        if let Some(uri) = a.at_uri.clone() {
            map.insert(uri, a);
        }
    }
    Ok(uris.iter().filter_map(|u| map.get(u).cloned()).collect())
}

// ---- Queries (bypass visibility — admin / internal) ----

pub async fn get_article_any_visibility(pool: &PgPool, uri: &str) -> crate::Result<Article> {
    sqlx::query_as::<_, Article>(&format!("{ARTICLE_BASE} WHERE l.at_uri = $1"))
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() })
}

pub async fn get_article_owner(pool: &PgPool, uri: &str) -> crate::Result<String> {
    if let Some((series_id, source_path)) = super::series_service::parse_chapter_uri(uri) {
        let repo_uri = super::series_service::series_repo_uri(pool, &series_id).await?;
        return sqlx::query_scalar::<_, String>(
            "SELECT author_did FROM articles WHERE repo_uri = $1 AND source_path = $2",
        )
            .bind(&repo_uri)
            .bind(&source_path)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() });
    }
    sqlx::query_scalar::<_, String>(
        "SELECT a.author_did FROM articles a \
         JOIN article_localizations l \
             ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
         WHERE l.at_uri = $1",
    )
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() })
}

/// For an article URI, resolve the PDS-facing `(subject, sectionRef)` pair.
/// - Standalone article: `(article_uri, None)`
/// - Series chapter:     `(series_at_uri, Some(chapter_tid))`
///
/// The `series_lexicon` is passed in to avoid this crate depending on
/// fx-atproto; the caller supplies `fx_atproto::lexicon::WORK`.
pub async fn resolve_subject_ref(
    pool: &PgPool,
    article_uri: &str,
    series_lexicon: &str,
) -> (String, Option<String>) {
    // Locate the article via any of its localizations' at_uris.
    let found: Option<(String, String, String)> = sqlx::query_as(
        "SELECT s.id, s.created_by, sa.source_path \
         FROM series s \
         JOIN series_articles sa ON sa.series_id = s.id \
         JOIN article_localizations l \
             ON l.repo_uri = sa.repo_uri AND l.source_path = sa.source_path \
         WHERE l.at_uri = $1 \
         LIMIT 1",
    )
    .bind(article_uri)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    if let Some((series_id, owner_did, source_path)) = found {
        // sectionRef is the chapter's source_path (the file within the repo).
        let series_uri = format!("at://{owner_did}/{series_lexicon}/{series_id}");
        (series_uri, Some(source_path))
    } else {
        (article_uri.to_string(), None)
    }
}

pub async fn get_content_format(pool: &PgPool, uri: &str) -> crate::Result<String> {
    // `nightboat-chapter://{series_id}/{source_path_urlencoded}` — series
    // chapters have no at_uri. Resolve via composite key instead.
    if let Some((series_id, source_path)) = super::series_service::parse_chapter_uri(uri) {
        let repo_uri = super::series_service::series_repo_uri(pool, &series_id).await?;
        return sqlx::query_scalar::<_, String>(
            "SELECT content_format::text FROM article_localizations \
             WHERE repo_uri = $1 AND source_path = $2 LIMIT 1",
        )
        .bind(&repo_uri)
        .bind(&source_path)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() });
    }
    sqlx::query_scalar::<_, String>(
        "SELECT content_format::text FROM article_localizations WHERE at_uri = $1",
    )
        .bind(uri)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::Error::NotFound { entity: "article", id: uri.to_string() })
}

pub async fn get_article_prereqs(pool: &PgPool, uri: &str, _locale: &str) -> crate::Result<Vec<ArticlePrereqRow>> {
    // Edges are tag-level: content_prereqs only stores tag_id (concept).
    // For URL compatibility, we surface a canonical label id as `tag_id`
    // — the frontend's `tagStore` handles per-locale sibling lookup.
    // content_prereqs.content_uri stores the synthetic article URI; resolve
    // the caller's URI (at_uri or `nightboat-chapter://...`) to that form.
    let Some(synth) = super::series_service::resolve_to_synthetic_uri(pool, uri).await? else {
        return Ok(vec![]);
    };
    let rows = sqlx::query_as::<_, ArticlePrereqRow>(
        "SELECT cp.tag_id AS tag_id, \
                cp.prereq_type, \
                tag_canonical_label(cp.tag_id) AS tag_name, \
                tag_label_map(cp.tag_id) AS tag_names \
         FROM content_prereqs cp \
         WHERE cp.content_uri = $1 \
         ORDER BY cp.prereq_type, cp.tag_id",
    )
    .bind(&synth)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Concepts this article touches without teaching (see content_related).
/// Returns just tag_ids; the frontend localizes via tagStore.
pub async fn get_article_related(pool: &PgPool, uri: &str) -> crate::Result<Vec<String>> {
    let Some(synth) = super::series_service::resolve_to_synthetic_uri(pool, uri).await? else {
        return Ok(vec![]);
    };
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT tag_id FROM content_related \
         WHERE content_uri = $1 \
         ORDER BY tag_id",
    )
    .bind(&synth)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Bulk-fetch all article tag mappings (with safety limit). `tag_id` in
/// the response is the canonical label id for each tag — the frontend
/// uses it both to display (via `tagStore.localize`) and to navigate.
pub async fn get_all_article_teaches(pool: &PgPool, limit: i64) -> crate::Result<Vec<ContentTeachRow>> {
    let rows = sqlx::query_as::<_, ContentTeachRow>(
        "SELECT ct.content_uri, \
                ct.tag_id AS tag_id, \
                tag_canonical_label(ct.tag_id) AS tag_name, \
                tag_label_map(ct.tag_id) as tag_names \
         FROM content_teaches ct \
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
        "SELECT cp.content_uri, \
                tag_canonical_label(cp.tag_id) AS tag_id, \
                cp.prereq_type, \
                tag_canonical_label(cp.tag_id) AS tag_name, \
                tag_label_map(cp.tag_id) as tag_names \
         FROM content_prereqs cp \
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
/// `resolved_summary` is the final description source (may be auto-extracted
/// from content, see `SummaryInput` in the server layer); `summary_html`
/// is the inline-rendered HTML cache for list views.
///
/// `content_manifest`: when provided, the localization row is created with
/// `content_storage='blob'` and the manifest JSONB. When `None`, falls back
/// to the default `'pijul'` storage.
pub async fn create_article(
    pool: &PgPool,
    did: &str,
    at_uri: &str,
    input: &CreateArticle,
    content_hash: &str,
    _translation_group: Option<String>, // legacy; new model tracks translation via file metadata
    visibility: &str,
    kind: ContentKind,
    question_uri: Option<&str>,
    resolved_summary: &str,
    summary_html: &str,
    content_manifest: Option<serde_json::Value>,
) -> crate::Result<Article> {
    let lang = input.lang.as_deref().unwrap_or("zh");
    let restricted = input.restricted.unwrap_or(false);
    let license = if restricted { "All-Rights-Reserved" } else {
        input.license.as_deref().unwrap_or("CC-BY-SA-4.0")
    };
    let category = input.category.as_deref().unwrap_or("general");
    let ext = match input.content_format.as_str() {
        "markdown" => "md",
        "html" => "html",
        _ => "typ",
    };

    // Branch: `translation_of` (caller passes a source-article at_uri) means
    // "add this as a new localization under the existing article", NOT
    // "create a new article with a translation_group link". Short-circuit to
    // add_translation_localization and return immediately.
    if let Some(source_uri) = input.translation_of.as_deref() {
        return add_translation_localization(
            pool, did, at_uri, input, content_hash,
            source_uri, lang, ext,
            resolved_summary, summary_html,
        ).await;
    }

    // Q&A is a NightBoat-local feature: questions and answers don't live in
    // pijul (admins must be able to freely merge), don't get published to
    // PDS (at_uri is NULL on the localization). Content is stored inline in
    // `article_localizations.content_body`. The `at_uri` the caller passed
    // in is repurposed as the synthetic local identifier (so notifications,
    // comment links, etc. keep working) — but we store it on the row as
    // content_storage = 'server_db', NOT as an ATProto record URI.
    if matches!(kind, ContentKind::Question | ContentKind::Answer) {
        return create_qa_server_only(
            pool, did, at_uri, input, kind, question_uri,
            visibility, license, category, lang, ext, content_hash,
            resolved_summary, summary_html,
        ).await;
    }

    // Standalone: (repo_uri = at_uri, source_path = main.<ext>). Series chapter
    // creation takes a different path via series_service::add_series_article.
    let source_path = format!("main.{ext}");
    let repo_uri = at_uri.to_string();
    let synthetic = format!("nightboat://article/{repo_uri}/{source_path}");

    // Resolve question_uri (at_uri of a question) to its composite key.
    let (q_repo, q_src) = if let Some(qu) = question_uri {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1",
        )
        .bind(qu)
        .fetch_optional(pool)
        .await?;
        row.map(|(r, s)| (Some(r), Some(s))).unwrap_or((None, None))
    } else {
        (None, None)
    };

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO articles (\
            repo_uri, source_path, author_did, license, prereq_threshold, \
            kind, category, visibility, restricted, \
            book_id, edition_id, course_id, book_chapter_id, course_session_id, \
            question_repo_uri, question_source_path \
         ) VALUES ($1, $2, $3, $4, 0.8, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
    )
    .bind(&repo_uri)
    .bind(&source_path)
    .bind(did)
    .bind(license)
    .bind(kind)
    .bind(category)
    .bind(visibility)
    .bind(restricted)
    .bind(input.target_book_id())
    .bind(input.target_edition_id())
    .bind(input.target_course_id())
    .bind(input.target_book_chapter_id())
    .bind(input.target_course_session_id())
    .bind(q_repo)
    .bind(q_src)
    .execute(&mut *tx)
    .await?;

    let (storage, manifest) = match content_manifest {
        Some(m) => ("blob", Some(m)),
        None => ("pijul", None),
    };
    sqlx::query(
        "INSERT INTO article_localizations (\
            repo_uri, source_path, lang, at_uri, file_path, \
            content_format, title, summary, summary_html, content_hash, \
            content_storage, content_manifest \
         ) VALUES ($1, $2, $3, $4, $2, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(&repo_uri)
    .bind(&source_path)
    .bind(lang)
    .bind(at_uri)
    .bind(input.content_format)
    .bind(&input.title)
    .bind(resolved_summary)
    .bind(summary_html)
    .bind(content_hash)
    .bind(storage)
    .bind(&manifest)
    .execute(&mut *tx)
    .await?;

    for input_ref in &input.tags {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, did).await?;
        sqlx::query(
            "INSERT INTO content_teaches (content_uri, tag_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic).bind(&tag_id)
        .execute(&mut *tx).await?;
    }

    for prereq in &input.prereqs {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, &prereq.tag_id, did).await?;
        sqlx::query(
            "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) \
             VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic).bind(&tag_id).bind(prereq.prereq_type.as_str())
        .execute(&mut *tx).await?;
    }

    for input_ref in &input.related {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, did).await?;
        sqlx::query(
            "INSERT INTO content_related (content_uri, tag_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic).bind(&tag_id)
        .execute(&mut *tx).await?;
    }

    for input_ref in &input.topics {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, did).await?;
        sqlx::query(
            "INSERT INTO content_topics (content_uri, tag_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic).bind(&tag_id)
        .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    let article = sqlx::query_as::<_, Article>(&format!("{ARTICLE_BASE} WHERE l.at_uri = $1"))
        .bind(at_uri)
        .fetch_one(pool)
        .await?;
    Ok(article)
}

/// Create a series chapter row directly. Unlike [`create_article`], the
/// chapter has NO per-chapter ATProto record — it lives inside its series's
/// `at.nightbo.work` record via `chapters[]` + `files[]`. The DB rows are
/// keyed by `(series_repo_uri, source_path)` with `at_uri = NULL`.
///
/// The `series_repo_uri` is the series's canonical URI
/// (`at://{did}/at.nightbo.work/{series_id}`). `source_path` is the
/// chapter's file within the series bundle (e.g. `chapters/01-intro.typ`).
pub async fn create_series_chapter(
    pool: &PgPool,
    did: &str,
    series_repo_uri: &str,
    source_path: &str,
    input: &CreateArticle,
    content_hash: &str,
    visibility: &str,
    resolved_summary: &str,
    summary_html: &str,
    content_manifest: Option<serde_json::Value>,
) -> crate::Result<Article> {
    let lang = input.lang.as_deref().unwrap_or("zh");
    let restricted = input.restricted.unwrap_or(false);
    let license = if restricted { "All-Rights-Reserved" } else {
        input.license.as_deref().unwrap_or("CC-BY-SA-4.0")
    };
    let category = input.category.as_deref().unwrap_or("general");

    let synthetic = format!("nightboat://article/{series_repo_uri}/{source_path}");

    let mut tx = pool.begin().await?;

    // Upsert-style: support re-publish of the same chapter by `(repo_uri, source_path)`.
    sqlx::query(
        "INSERT INTO articles (\
            repo_uri, source_path, author_did, license, prereq_threshold, \
            kind, category, visibility, restricted, \
            book_id, edition_id, course_id, book_chapter_id, course_session_id \
         ) VALUES ($1, $2, $3, $4, 0.8, 'article', $5, $6, $7, $8, $9, $10, $11, $12) \
         ON CONFLICT (repo_uri, source_path) DO UPDATE SET \
             license = EXCLUDED.license, \
             category = EXCLUDED.category, \
             visibility = EXCLUDED.visibility, \
             restricted = EXCLUDED.restricted, \
             book_id = EXCLUDED.book_id, \
             edition_id = EXCLUDED.edition_id, \
             course_id = EXCLUDED.course_id, \
             book_chapter_id = EXCLUDED.book_chapter_id, \
             course_session_id = EXCLUDED.course_session_id, \
             updated_at = NOW()",
    )
    .bind(series_repo_uri)
    .bind(source_path)
    .bind(did)
    .bind(license)
    .bind(category)
    .bind(visibility)
    .bind(restricted)
    .bind(input.target_book_id())
    .bind(input.target_edition_id())
    .bind(input.target_course_id())
    .bind(input.target_book_chapter_id())
    .bind(input.target_course_session_id())
    .execute(&mut *tx)
    .await?;

    let (storage, manifest) = match content_manifest {
        Some(m) => ("blob", Some(m)),
        None => ("pijul", None),
    };

    // Chapter localization: file_path = source_path (source lang for now;
    // per-chapter translations can be added later as extra rows).
    sqlx::query(
        "INSERT INTO article_localizations (\
            repo_uri, source_path, lang, at_uri, file_path, \
            content_format, title, summary, summary_html, content_hash, \
            content_storage, content_manifest \
         ) VALUES ($1, $2, $3, NULL, $2, $4, $5, $6, $7, $8, $9, $10) \
         ON CONFLICT (repo_uri, source_path, lang) DO UPDATE SET \
             file_path = EXCLUDED.file_path, \
             content_format = EXCLUDED.content_format, \
             title = EXCLUDED.title, \
             summary = EXCLUDED.summary, \
             summary_html = EXCLUDED.summary_html, \
             content_hash = EXCLUDED.content_hash, \
             content_storage = EXCLUDED.content_storage, \
             content_manifest = EXCLUDED.content_manifest, \
             updated_at = NOW()",
    )
    .bind(series_repo_uri)
    .bind(source_path)
    .bind(lang)
    .bind(input.content_format)
    .bind(&input.title)
    .bind(resolved_summary)
    .bind(summary_html)
    .bind(content_hash)
    .bind(storage)
    .bind(&manifest)
    .execute(&mut *tx)
    .await?;

    // Refresh teaches/prereqs/related/topics for this chapter — re-publish
    // overwrites by clearing and re-inserting.
    sqlx::query("DELETE FROM content_teaches WHERE content_uri = $1")
        .bind(&synthetic).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM content_prereqs WHERE content_uri = $1")
        .bind(&synthetic).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM content_related WHERE content_uri = $1")
        .bind(&synthetic).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM content_topics WHERE content_uri = $1")
        .bind(&synthetic).execute(&mut *tx).await?;

    for input_ref in &input.tags {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, did).await?;
        sqlx::query(
            "INSERT INTO content_teaches (content_uri, tag_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic).bind(&tag_id)
        .execute(&mut *tx).await?;
    }

    for prereq in &input.prereqs {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, &prereq.tag_id, did).await?;
        sqlx::query(
            "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) \
             VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic).bind(&tag_id).bind(prereq.prereq_type.as_str())
        .execute(&mut *tx).await?;
    }

    for input_ref in &input.related {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, did).await?;
        sqlx::query(
            "INSERT INTO content_related (content_uri, tag_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic).bind(&tag_id)
        .execute(&mut *tx).await?;
    }

    for input_ref in &input.topics {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, did).await?;
        sqlx::query(
            "INSERT INTO content_topics (content_uri, tag_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic).bind(&tag_id)
        .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    // Return the Article view of this chapter — keyed by composite not at_uri
    // (which is NULL). Use ARTICLE_BASE_ANY_LANG without the source-lang pin.
    let sql = format!(
        "{ARTICLE_BASE_ANY_LANG} WHERE a.repo_uri = $1 AND a.source_path = $2 AND l.lang = $3"
    );
    let article = sqlx::query_as::<_, Article>(&sql)
        .bind(series_repo_uri)
        .bind(source_path)
        .bind(lang)
        .fetch_one(pool)
        .await?;
    Ok(article)
}

/// Q&A local-only publish path: no pijul, no PDS record. Creates an
/// `articles` row with synthetic `repo_uri = server://qa/{rkey}` and an
/// `article_localizations` row with `content_storage = 'server_db'` +
/// `content_body = <input.content>`, `at_uri = NULL`.
///
/// `at_uri` arg is the ATProto-style URI the caller minted (for historical
/// compatibility with legacy rkeys) — we extract the rkey from it to form
/// the server-local `repo_uri`. We do NOT store it as a real at_uri on the
/// localization since Q&A is not federated.
async fn create_qa_server_only(
    pool: &PgPool,
    did: &str,
    at_uri: &str,
    input: &CreateArticle,
    kind: ContentKind,
    question_uri: Option<&str>,
    visibility: &str,
    license: &str,
    category: &str,
    lang: &str,
    ext: &str,
    content_hash: &str,
    resolved_summary: &str,
    summary_html: &str,
) -> crate::Result<Article> {
    let rkey = at_uri.rsplit('/').next().unwrap_or("unknown");
    let repo_uri = format!("server://qa/{rkey}");
    let source_path = match kind {
        ContentKind::Question => format!("question.{ext}"),
        ContentKind::Answer   => format!("answer.{ext}"),
        _ => unreachable!("create_qa_server_only called with non-Q&A kind"),
    };
    let synthetic_ref = format!("nightboat://article/{repo_uri}/{source_path}");

    // Resolve question_uri (for answers) to composite key. Callers pass it as
    // the question article's synthetic URI or legacy at_uri; we accept either.
    let (q_repo, q_src) = if let Some(qu) = question_uri {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT repo_uri, source_path FROM articles a \
             WHERE article_uri(a.repo_uri, a.source_path) = $1 \
             UNION ALL \
             SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1 \
             LIMIT 1",
        )
        .bind(qu)
        .fetch_optional(pool)
        .await?;
        row.map(|(r, s)| (Some(r), Some(s))).unwrap_or((None, None))
    } else {
        (None, None)
    };

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO articles (\
            repo_uri, source_path, author_did, license, prereq_threshold, \
            kind, category, visibility, restricted, \
            book_id, edition_id, course_id, book_chapter_id, course_session_id, \
            question_repo_uri, question_source_path \
         ) VALUES ($1, $2, $3, $4, 0.8, $5, $6, $7, FALSE, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(&repo_uri)
    .bind(&source_path)
    .bind(did)
    .bind(license)
    .bind(kind)
    .bind(category)
    .bind(visibility)
    .bind(input.target_book_id())
    .bind(input.target_edition_id())
    .bind(input.target_course_id())
    .bind(input.target_book_chapter_id())
    .bind(input.target_course_session_id())
    .bind(q_repo)
    .bind(q_src)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO article_localizations (\
            repo_uri, source_path, lang, at_uri, file_path, \
            content_format, title, summary, summary_html, content_hash, \
            content_storage, content_body \
         ) VALUES ($1, $2, $3, NULL, $2, $4, $5, $6, $7, $8, 'server_db', $9)",
    )
    .bind(&repo_uri)
    .bind(&source_path)
    .bind(lang)
    .bind(input.content_format)
    .bind(&input.title)
    .bind(resolved_summary)
    .bind(summary_html)
    .bind(content_hash)
    .bind(&input.content)
    .execute(&mut *tx)
    .await?;

    // Tags/prereqs/related/topics keyed on the synthetic article URI.
    for input_ref in &input.tags {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, did).await?;
        sqlx::query(
            "INSERT INTO content_teaches (content_uri, tag_id) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic_ref).bind(&tag_id)
        .execute(&mut *tx).await?;
    }
    for prereq in &input.prereqs {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, &prereq.tag_id, did).await?;
        sqlx::query(
            "INSERT INTO content_prereqs (content_uri, tag_id, prereq_type) \
             VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic_ref).bind(&tag_id).bind(prereq.prereq_type.as_str())
        .execute(&mut *tx).await?;
    }
    for input_ref in &input.related {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, did).await?;
        sqlx::query(
            "INSERT INTO content_related (content_uri, tag_id) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic_ref).bind(&tag_id)
        .execute(&mut *tx).await?;
    }
    for input_ref in &input.topics {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, did).await?;
        sqlx::query(
            "INSERT INTO content_topics (content_uri, tag_id) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic_ref).bind(&tag_id)
        .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    // Return the article view. Q&A rows have at_uri NULL, so we can't use
    // `WHERE l.at_uri = $1`; look up by composite key directly.
    let article = sqlx::query_as::<_, Article>(
        &format!("{ARTICLE_BASE} WHERE a.repo_uri = $1 AND a.source_path = $2"),
    )
    .bind(&repo_uri)
    .bind(&source_path)
    .fetch_one(pool)
    .await?;
    Ok(article)
}

/// Add a new localization under an existing article. `source_uri` is any of
/// the source article's localization at_uris; we resolve it to
/// `(repo_uri, source_path)` and insert a new row in `article_localizations`
/// under the same key but a different `lang`.
///
/// The new localization's `file_path` defaults to `{stem}.{lang}.{ext}` as a
/// sibling of the source file — matching the `translation_of: ./source.md`
/// convention recognized by the validator's file-level metadata.
async fn add_translation_localization(
    pool: &PgPool,
    translator_did: &str,
    at_uri: &str,
    input: &CreateArticle,
    content_hash: &str,
    source_uri: &str,
    lang: &str,
    ext: &str,
    resolved_summary: &str,
    summary_html: &str,
) -> crate::Result<Article> {
    // Resolve source → (repo_uri, source_path).
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1",
    )
    .bind(source_uri)
    .fetch_optional(pool)
    .await?;
    let Some((repo_uri, source_path)) = row else {
        return Err(crate::Error::NotFound {
            entity: "translation source",
            id: source_uri.to_string(),
        });
    };

    // Derive a default file_path: strip the source's extension, append
    // `.{lang}.{ext}`. Example: `main.typ` + `zh-CN` → `main.zh-CN.typ`.
    let file_path = {
        let stem = match source_path.rsplit_once('.') {
            Some((s, _)) => s,
            None => source_path.as_str(),
        };
        format!("{stem}.{lang}.{ext}")
    };

    sqlx::query(
        "INSERT INTO article_localizations (\
            repo_uri, source_path, lang, at_uri, file_path, \
            content_format, title, summary, summary_html, content_hash, \
            translator_did \
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         ON CONFLICT (repo_uri, source_path, lang) DO UPDATE SET \
             at_uri = EXCLUDED.at_uri, \
             file_path = EXCLUDED.file_path, \
             content_format = EXCLUDED.content_format, \
             title = EXCLUDED.title, \
             summary = EXCLUDED.summary, \
             summary_html = EXCLUDED.summary_html, \
             content_hash = EXCLUDED.content_hash, \
             translator_did = EXCLUDED.translator_did, \
             updated_at = NOW()",
    )
    .bind(&repo_uri)
    .bind(&source_path)
    .bind(lang)
    .bind(at_uri)
    .bind(&file_path)
    .bind(input.content_format)
    .bind(&input.title)
    .bind(resolved_summary)
    .bind(summary_html)
    .bind(content_hash)
    .bind(translator_did)
    .execute(pool)
    .await?;

    // Return the Article view of this specific localization. We can't reuse
    // ARTICLE_BASE here: it pins `l.file_path = a.source_path` to yield only
    // the source-language row, but we just inserted a translation whose
    // file_path ≠ source_path. Run a custom query without that constraint.
    let article = sqlx::query_as::<_, Article>(
        "SELECT a.repo_uri, a.source_path, a.author_did, \
         p.handle AS author_handle, p.display_name AS author_display_name, \
         p.avatar_url AS author_avatar, COALESCE(p.reputation, 0) AS author_reputation, \
         a.kind, a.category, a.license, a.prereq_threshold, a.restricted, a.answer_count, \
         a.cover_url, a.cover_file, \
         a.question_repo_uri, a.question_source_path, \
         a.book_id, a.edition_id, a.course_id, a.book_chapter_id, a.course_session_id, \
         l.lang, l.at_uri, l.file_path, l.title, l.summary, l.summary_html, \
         l.content_hash, l.content_format, l.translator_did, \
         pm.venue AS paper_venue, pm.year AS paper_year, pm.accepted AS paper_accepted, \
         COALESCE(v.vote_score, 0) AS vote_score, \
         COALESCE(b.bookmark_count, 0) AS bookmark_count, \
         COALESCE(cm.comment_count, 0) AS comment_count, \
         a.created_at, a.updated_at \
         FROM articles a \
         JOIN article_localizations l \
             ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
         LEFT JOIN profiles p ON p.did = a.author_did \
         LEFT JOIN paper_metadata pm \
             ON pm.repo_uri = a.repo_uri AND pm.source_path = a.source_path \
         LEFT JOIN (SELECT target_uri, SUM(value) AS vote_score FROM votes GROUP BY target_uri) v \
             ON v.target_uri = article_uri(a.repo_uri, a.source_path) \
         LEFT JOIN (SELECT repo_uri, source_path, COUNT(*) AS bookmark_count FROM user_bookmarks GROUP BY repo_uri, source_path) b \
             ON b.repo_uri = a.repo_uri AND b.source_path = a.source_path \
         LEFT JOIN (SELECT content_uri, COUNT(*) AS comment_count FROM comments GROUP BY content_uri) cm \
             ON cm.content_uri = article_uri(a.repo_uri, a.source_path) \
         WHERE l.at_uri = $1",
    )
        .bind(at_uri)
        .fetch_one(pool)
        .await?;
    Ok(article)
}

pub async fn update_article_title(pool: &PgPool, uri: &str, title: &str) -> crate::Result<()> {
    sqlx::query("UPDATE article_localizations SET title = $1, updated_at = NOW() WHERE at_uri = $2")
        .bind(title).bind(uri).execute(pool).await?;
    Ok(())
}

pub async fn update_article_summary(
    pool: &PgPool, uri: &str, desc: &str, desc_html: &str,
) -> crate::Result<()> {
    sqlx::query(
        "UPDATE article_localizations SET summary = $1, summary_html = $2, updated_at = NOW() WHERE at_uri = $3",
    )
    .bind(desc).bind(desc_html).bind(uri).execute(pool).await?;
    Ok(())
}

pub async fn update_article_content_hash(pool: &PgPool, uri: &str, hash: &str) -> crate::Result<()> {
    sqlx::query("UPDATE article_localizations SET content_hash = $1, updated_at = NOW() WHERE at_uri = $2")
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
                 WHERE (repo_uri, source_path) IN \
                     (SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1) \
                 AND visibility != 'removed'",
            )
            .bind(uri).bind(reason)
            .execute(pool).await?
        }
        _ => {
            sqlx::query(
                "UPDATE articles SET visibility = $2, removed_at = NULL, remove_reason = NULL, updated_at = NOW() \
                 WHERE (repo_uri, source_path) IN \
                     (SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1)",
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
    resolved_summary: &str,
    summary_html: &str,
) -> crate::Result<Article> {
    let lang = input.lang.as_deref().unwrap_or("zh");
    let license = input.license.as_deref().unwrap_or("CC-BY-SA-4.0");
    let category = input.category.as_deref().unwrap_or("general");

    let mut tx = pool.begin().await?;

    // Article-level fields
    sqlx::query(
        "UPDATE articles SET license = $2, category = $3, updated_at = NOW() \
         WHERE (repo_uri, source_path) IN \
             (SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1)",
    )
    .bind(uri)
    .bind(license)
    .bind(category)
    .execute(&mut *tx)
    .await?;

    // Localization-level fields
    sqlx::query(
        "UPDATE article_localizations \
         SET title = $2, summary = $3, summary_html = $4, \
             content_hash = $5, content_format = $6, lang = $7, updated_at = NOW() \
         WHERE at_uri = $1",
    )
    .bind(uri)
    .bind(&input.title)
    .bind(resolved_summary)
    .bind(summary_html)
    .bind(content_hash)
    .bind(input.content_format)
    .bind(lang)
    .execute(&mut *tx)
    .await?;

    // content_teaches keys by synthetic article URI now.
    let synthetic: String = sqlx::query_scalar(
        "SELECT article_uri(a.repo_uri, a.source_path) \
         FROM articles a \
         JOIN article_localizations l \
             ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
         WHERE l.at_uri = $1",
    )
    .bind(uri)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM content_teaches WHERE content_uri = $1")
        .bind(&synthetic).execute(&mut *tx).await?;
    let updater_did: String = sqlx::query_scalar(
        "SELECT a.author_did FROM articles a \
         JOIN article_localizations l \
             ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
         WHERE l.at_uri = $1",
    )
        .bind(uri)
        .fetch_one(&mut *tx)
        .await?;
    for input_ref in &input.tags {
        let tag_id = crate::services::tag_service::resolve_tag_id(&mut *tx, input_ref, &updater_did).await?;
        sqlx::query(
            "INSERT INTO content_teaches (content_uri, tag_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(&synthetic).bind(&tag_id)
        .execute(&mut *tx).await?;
    }

    tx.commit().await?;

    let article = sqlx::query_as::<_, Article>(&format!("{ARTICLE_BASE} WHERE l.at_uri = $1"))
        .bind(uri)
        .fetch_one(pool)
        .await?;
    Ok(article)
}

/// Hard-delete the article identified by one of its localizations' at_uri.
/// Cascades to all localizations and FK-dependent tables; votes keyed by the
/// synthetic article URI are cleaned up manually.
pub async fn delete_article(pool: &PgPool, uri: &str) -> crate::Result<()> {
    let mut tx = pool.begin().await?;

    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT a.repo_uri, a.source_path, article_uri(a.repo_uri, a.source_path) \
         FROM articles a \
         JOIN article_localizations l \
             ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
         WHERE l.at_uri = $1",
    )
    .bind(uri)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((repo_uri, source_path, synthetic)) = row else {
        return Ok(());
    };

    sqlx::query("DELETE FROM votes WHERE target_uri = $1")
        .bind(&synthetic).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM articles WHERE repo_uri = $1 AND source_path = $2")
        .bind(&repo_uri).bind(&source_path).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Hard-delete articles removed more than 30 days ago.
pub async fn cleanup_expired_removals(pool: &PgPool) -> crate::Result<u64> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT repo_uri, source_path FROM articles \
         WHERE visibility = 'removed' AND removed_at < NOW() - INTERVAL '30 days'",
    )
    .fetch_all(pool).await?;

    if rows.is_empty() {
        return Ok(0);
    }

    let synthetic_uris: Vec<String> = rows
        .iter()
        .map(|(r, s)| format!("nightboat://article/{r}/{s}"))
        .collect();

    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM votes WHERE target_uri = ANY($1)")
        .bind(&synthetic_uris).execute(&mut *tx).await?;
    let mut deleted = 0u64;
    for (repo, src) in &rows {
        let r = sqlx::query("DELETE FROM articles WHERE repo_uri = $1 AND source_path = $2")
            .bind(repo).bind(src)
            .execute(&mut *tx).await?;
        deleted += r.rows_affected();
    }
    tx.commit().await?;
    Ok(deleted)
}

/// Legacy shim: translation_group is gone; in the new model translations are
/// siblings in `article_localizations` keyed by file-level metadata. Returns
/// the source URI unchanged so publish-flow callers keep compiling until they
/// are rewritten.
#[deprecated(note = "translation_group was removed; rewrite caller against article_localizations")]
pub async fn resolve_translation_group(_pool: &PgPool, source_uri: &str) -> crate::Result<String> {
    Ok(source_uri.to_string())
}

/// Merge question `from_uri` into `into_uri`: move all answers, log merge, delete old question.
///
/// Q&A will move to a server-only model (task #9). Until then, this function
/// still takes at_uri-style strings and resolves through article_localizations.
pub async fn merge_questions(pool: &PgPool, from_uri: &str, into_uri: &str) -> crate::Result<u64> {
    let mut tx = pool.begin().await?;

    let from_key: Option<(String, String)> = sqlx::query_as(
        "SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1",
    )
    .bind(from_uri).fetch_optional(&mut *tx).await?;
    let into_key: Option<(String, String)> = sqlx::query_as(
        "SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1",
    )
    .bind(into_uri).fetch_optional(&mut *tx).await?;

    let (Some((from_r, from_s)), Some((into_r, into_s))) = (from_key, into_key) else {
        return Ok(0);
    };

    // Repoint answers
    let result = sqlx::query(
        "UPDATE articles SET question_repo_uri = $3, question_source_path = $4 \
         WHERE question_repo_uri = $1 AND question_source_path = $2 AND kind = 'answer'",
    )
    .bind(&from_r).bind(&from_s).bind(&into_r).bind(&into_s)
    .execute(&mut *tx).await?;
    let moved = result.rows_affected();

    // Recount answer_count on the destination question
    sqlx::query(
        "UPDATE articles SET answer_count = \
             (SELECT COUNT(*) FROM articles \
              WHERE question_repo_uri = $1 AND question_source_path = $2 AND kind = 'answer') \
         WHERE repo_uri = $1 AND source_path = $2",
    )
    .bind(&into_r).bind(&into_s)
    .execute(&mut *tx).await?;

    sqlx::query(
        "INSERT INTO question_merges (from_uri, into_repo_uri, into_source_path) \
         VALUES ($1, $2, $3) ON CONFLICT (from_uri) DO NOTHING",
    )
    .bind(from_uri).bind(&into_r).bind(&into_s)
    .execute(&mut *tx).await?;

    delete_article_in_tx(&mut tx, &from_r, &from_s).await?;

    tx.commit().await?;
    Ok(moved)
}

async fn delete_article_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    repo_uri: &str,
    source_path: &str,
) -> crate::Result<()> {
    let synthetic = format!("nightboat://article/{repo_uri}/{source_path}");
    sqlx::query("DELETE FROM votes WHERE target_uri = $1")
        .bind(&synthetic).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM articles WHERE repo_uri = $1 AND source_path = $2")
        .bind(repo_uri).bind(source_path).execute(&mut **tx).await?;
    Ok(())
}

/// Auto-bookmark an article into the user's folder. `uri` is a localization's
/// at_uri; resolved to the composite article key before insert.
pub async fn auto_bookmark(pool: &PgPool, did: &str, uri: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO user_bookmarks (did, repo_uri, source_path, folder_path) \
         SELECT $1, repo_uri, source_path, '我的文章' \
         FROM article_localizations WHERE at_uri = $2 \
         ON CONFLICT DO NOTHING",
    )
    .bind(did).bind(uri)
    .execute(pool).await?;
    Ok(())
}

// --- Access control (paywall) ---

/// Check if a viewer can access article content. `uri` is either a
/// localization's at_uri, or the synthetic `nightboat-chapter://` URI for a
/// series chapter (which has no at_uri).
pub async fn check_content_access(pool: &PgPool, uri: &str, viewer_did: Option<&str>) -> crate::Result<bool> {
    let row: Option<(String, String, String, bool)> =
        if let Some((series_id, source_path)) = super::series_service::parse_chapter_uri(uri) {
            let repo_uri = super::series_service::series_repo_uri(pool, &series_id).await?;
            sqlx::query_as(
                "SELECT a.author_did, a.repo_uri, a.source_path, a.restricted \
                 FROM articles a \
                 WHERE a.repo_uri = $1 AND a.source_path = $2",
            )
            .bind(&repo_uri).bind(&source_path)
            .fetch_optional(pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT a.author_did, a.repo_uri, a.source_path, a.restricted \
                 FROM articles a \
                 JOIN article_localizations l \
                     ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
                 WHERE l.at_uri = $1",
            )
            .bind(uri)
            .fetch_optional(pool)
            .await?
        };

    let Some((owner_did, repo_uri, source_path, restricted)) = row else {
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
            SELECT 1 FROM article_access_grants \
                WHERE repo_uri = $1 AND source_path = $2 AND grantee_did = $3 \
            UNION ALL \
            SELECT 1 FROM user_members WHERE author_did = $4 AND member_did = $3 \
        )",
    )
    .bind(&repo_uri).bind(&source_path).bind(viewer).bind(&owner_did)
    .fetch_one(pool)
    .await?;

    Ok(granted)
}

pub async fn set_restricted(pool: &PgPool, uri: &str, restricted: bool) -> crate::Result<()> {
    let sql = if restricted {
        "UPDATE articles SET restricted = TRUE, license = 'All-Rights-Reserved', updated_at = NOW() \
         WHERE (repo_uri, source_path) IN \
             (SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1)"
    } else {
        "UPDATE articles SET restricted = FALSE, updated_at = NOW() \
         WHERE (repo_uri, source_path) IN \
             (SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1)"
    };
    sqlx::query(sql).bind(uri).execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct AccessGrant {
    pub repo_uri: String,
    pub source_path: String,
    pub grantee_did: String,
    pub granted_at: chrono::DateTime<chrono::Utc>,
}

pub async fn grant_access(pool: &PgPool, uri: &str, grantee_did: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO article_access_grants (repo_uri, source_path, grantee_did) \
         SELECT repo_uri, source_path, $2 FROM article_localizations WHERE at_uri = $1 \
         ON CONFLICT DO NOTHING",
    )
    .bind(uri).bind(grantee_did)
    .execute(pool).await?;
    Ok(())
}

pub async fn revoke_access(pool: &PgPool, uri: &str, grantee_did: &str) -> crate::Result<()> {
    sqlx::query(
        "DELETE FROM article_access_grants \
         WHERE (repo_uri, source_path) IN \
             (SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1) \
         AND grantee_did = $2",
    )
        .bind(uri).bind(grantee_did).execute(pool).await?;
    Ok(())
}

pub async fn list_access_grants(pool: &PgPool, uri: &str) -> crate::Result<Vec<AccessGrant>> {
    let rows = sqlx::query_as::<_, AccessGrant>(
        "SELECT repo_uri, source_path, grantee_did, granted_at \
         FROM article_access_grants \
         WHERE (repo_uri, source_path) IN \
             (SELECT repo_uri, source_path FROM article_localizations WHERE at_uri = $1) \
         ORDER BY granted_at",
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
        "INSERT INTO paper_metadata (repo_uri, source_path, venue, venue_type, year, doi, arxiv_id, accepted) \
         SELECT repo_uri, source_path, $2, $3, $4, $5, $6, $7 \
         FROM article_localizations WHERE at_uri = $1 \
         ON CONFLICT (repo_uri, source_path) DO UPDATE SET \
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
        "INSERT INTO experience_metadata (repo_uri, source_path, kind, target, year, result) \
         SELECT repo_uri, source_path, $2, $3, $4, $5 \
         FROM article_localizations WHERE at_uri = $1 \
         ON CONFLICT (repo_uri, source_path) DO UPDATE SET \
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
        "SELECT em.repo_uri, em.source_path, em.kind, em.target, em.year, em.result \
         FROM experience_metadata em \
         JOIN article_localizations l \
             ON l.repo_uri = em.repo_uri AND l.source_path = em.source_path \
         WHERE l.at_uri = $1",
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
        "SELECT pm.repo_uri, pm.source_path, pm.venue, pm.venue_type, pm.year, pm.doi, pm.arxiv_id, pm.accepted \
         FROM paper_metadata pm \
         JOIN article_localizations l \
             ON l.repo_uri = pm.repo_uri AND l.source_path = pm.source_path \
         WHERE l.at_uri = $1",
    )
    .bind(article_uri)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_questions_by_session(
    pool: &PgPool,
    mode: InstanceMode,
    session_id: &str,
    limit: i64,
) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'question' AND a.course_session_id = $1 \
         ORDER BY vote_score DESC, a.created_at DESC LIMIT $2",
        visible(mode)
    ))
    .bind(session_id).bind(limit)
    .fetch_all(pool).await?;
    Ok(rows)
}

pub async fn get_questions_by_homework(
    pool: &PgPool,
    mode: InstanceMode,
    homework_id: &str,
    limit: i64,
) -> crate::Result<Vec<Article>> {
    let rows = sqlx::query_as::<_, Article>(&format!(
        "{} AND a.kind = 'question' AND a.homework_id = $1 \
         ORDER BY vote_score DESC, a.created_at DESC LIMIT $2",
        visible(mode)
    ))
    .bind(homework_id).bind(limit)
    .fetch_all(pool).await?;
    Ok(rows)
}
