//! Papers: first-class entity parallel to books.
//!
//! A paper is the durable identity of a scholarly work. Multiple
//! `paper_versions` rows attach the various artefacts (arxiv preprint,
//! publisher PDF, camera-ready, NightBoat-native article body) without
//! splintering the discussion thread, which lives at the paper level via
//! `content_uri = 'paper:{id}'`.
//!
//! See `migrations_pg/20260425000006_papers_first_class.sql` for the schema
//! rationale and the back-fill from `paper_metadata`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Paper {
    pub id: String,
    #[ts(type = "Record<string, string>")]
    pub title: sqlx::types::Json<std::collections::HashMap<String, String>>,
    #[ts(type = "Record<string, string>")]
    pub abstract_: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub authors: Vec<String>,
    pub venue: Option<String>,
    pub venue_kind: Option<String>,
    pub year: Option<i16>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub bibtex_key: Option<String>,
    pub accepted: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PaperVersion {
    pub id: String,
    pub paper_id: String,
    /// `preprint`, `accepted`, `published`, `native`, `other`. `native`
    /// versions point at a NightBoat-hosted article (the canonical body);
    /// every other kind links out via `url`.
    pub kind: String,
    pub url: Option<String>,
    pub article_uri: Option<String>,
    pub year: Option<i16>,
    pub label: Option<String>,
    pub sort_order: i16,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PaperAuthor {
    pub author_id: String,
    pub name: String,
    pub orcid: Option<String>,
    pub affiliation: Option<String>,
    pub position: i16,
    pub role: String,
}

/// One paper as it appears in the list view: enough metadata for a card,
/// plus an aggregate vote / comment count.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PaperListItem {
    pub id: String,
    #[ts(type = "Record<string, string>")]
    pub title: sqlx::types::Json<std::collections::HashMap<String, String>>,
    pub authors: Vec<String>,
    pub venue: Option<String>,
    pub year: Option<i16>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub created_at: DateTime<Utc>,
    #[sqlx(default)]
    pub vote_score: i64,
    #[sqlx(default)]
    pub comment_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PaperDetailResponse {
    pub paper: Paper,
    pub versions: Vec<PaperVersion>,
    pub authors_detail: Vec<PaperAuthor>,
    /// Comment count keyed off `content_uri = 'paper:{id}'`.
    pub comment_count: i64,
    pub vote_score: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreatePaper {
    #[ts(type = "Record<string, string>")]
    pub title: std::collections::HashMap<String, String>,
    #[serde(default)]
    #[ts(type = "Record<string, string> | undefined")]
    pub abstract_: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub venue: Option<String>,
    #[serde(default)]
    pub venue_kind: Option<String>,
    #[serde(default)]
    pub year: Option<i16>,
    #[serde(default)]
    pub doi: Option<String>,
    #[serde(default)]
    pub arxiv_id: Option<String>,
    #[serde(default)]
    pub bibtex_key: Option<String>,
    #[serde(default = "default_true")]
    pub accepted: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateVersion {
    pub kind: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub article_uri: Option<String>,
    #[serde(default)]
    pub year: Option<i16>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub sort_order: Option<i16>,
}

// ── Reads ──────────────────────────────────────────────────────────────────

pub async fn get_paper(pool: &PgPool, id: &str) -> crate::Result<Paper> {
    sqlx::query_as::<_, Paper>(
        "SELECT id, title, abstract AS abstract_, authors, venue, venue_kind, \
                year, doi, arxiv_id, bibtex_key, accepted, created_by, created_at \
         FROM papers WHERE id = $1 AND removed_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(Error::NotFound { entity: "paper", id: id.to_string() })
}

pub async fn list_versions(pool: &PgPool, paper_id: &str) -> crate::Result<Vec<PaperVersion>> {
    let rows = sqlx::query_as::<_, PaperVersion>(
        "SELECT id, paper_id, kind, url, article_uri, year, label, sort_order, created_at \
         FROM paper_versions WHERE paper_id = $1 \
         ORDER BY sort_order, created_at",
    )
    .bind(paper_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_authors(pool: &PgPool, paper_id: &str) -> crate::Result<Vec<PaperAuthor>> {
    let rows = sqlx::query_as::<_, PaperAuthor>(
        "SELECT pa.author_id, a.name, a.orcid, a.affiliation, pa.position, pa.role \
         FROM paper_authors pa \
         JOIN authors a ON a.id = pa.author_id \
         WHERE pa.paper_id = $1 \
         ORDER BY pa.position, a.name",
    )
    .bind(paper_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_paper_detail(pool: &PgPool, id: &str) -> crate::Result<PaperDetailResponse> {
    let paper = get_paper(pool, id).await?;
    let versions = list_versions(pool, id).await?;
    let authors_detail = list_authors(pool, id).await?;

    // Counts keyed off `paper:{id}` content URI.
    let content_uri = format!("paper:{id}");
    let vote_score: Option<i64> = sqlx::query_scalar(
        "SELECT SUM(value) FROM votes WHERE target_uri = $1",
    )
    .bind(&content_uri)
    .fetch_optional(pool)
    .await?
    .flatten();
    let comment_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM comments WHERE content_uri = $1",
    )
    .bind(&content_uri)
    .fetch_one(pool)
    .await?;

    Ok(PaperDetailResponse {
        paper,
        versions,
        authors_detail,
        vote_score: vote_score.unwrap_or(0),
        comment_count,
    })
}

pub async fn list_papers(pool: &PgPool, limit: i64, offset: i64) -> crate::Result<Vec<PaperListItem>> {
    let rows = sqlx::query_as::<_, PaperListItem>(
        "SELECT p.id, p.title, p.authors, p.venue, p.year, p.doi, p.arxiv_id, p.created_at, \
                COALESCE(v.score, 0) AS vote_score, \
                COALESCE(c.cnt,    0) AS comment_count \
         FROM papers p \
         LEFT JOIN (SELECT target_uri, SUM(value) AS score FROM votes GROUP BY target_uri) v \
                ON v.target_uri = 'paper:' || p.id \
         LEFT JOIN (SELECT content_uri, COUNT(*) AS cnt FROM comments GROUP BY content_uri) c \
                ON c.content_uri = 'paper:' || p.id \
         WHERE p.removed_at IS NULL \
         ORDER BY p.created_at DESC \
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ── Writes ─────────────────────────────────────────────────────────────────

pub async fn create_paper(
    pool: &PgPool,
    id: &str,
    created_by: &str,
    input: &CreatePaper,
) -> crate::Result<Paper> {
    let title_json = sqlx::types::Json(input.title.clone());
    let abstract_json = sqlx::types::Json(
        input.abstract_.clone().unwrap_or_default(),
    );

    let mut tx = pool.begin().await?;

    let paper = sqlx::query_as::<_, Paper>(
        "INSERT INTO papers (id, title, abstract, authors, venue, venue_kind, \
                             year, doi, arxiv_id, bibtex_key, accepted, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
         RETURNING id, title, abstract AS abstract_, authors, venue, venue_kind, \
                   year, doi, arxiv_id, bibtex_key, accepted, created_by, created_at",
    )
    .bind(id)
    .bind(title_json)
    .bind(abstract_json)
    .bind(&input.authors)
    .bind(&input.venue)
    .bind(&input.venue_kind)
    .bind(input.year)
    .bind(&input.doi)
    .bind(&input.arxiv_id)
    .bind(&input.bibtex_key)
    .bind(input.accepted)
    .bind(created_by)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO content (uri, content_type) VALUES ($1, 'paper') ON CONFLICT DO NOTHING",
    )
    .bind(format!("paper:{id}"))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(paper)
}

pub async fn add_version(
    pool: &PgPool,
    paper_id: &str,
    version_id: &str,
    input: &CreateVersion,
) -> crate::Result<PaperVersion> {
    let row = sqlx::query_as::<_, PaperVersion>(
        "INSERT INTO paper_versions (id, paper_id, kind, url, article_uri, year, label, sort_order) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, COALESCE($8, 0)) \
         RETURNING id, paper_id, kind, url, article_uri, year, label, sort_order, created_at",
    )
    .bind(version_id)
    .bind(paper_id)
    .bind(&input.kind)
    .bind(&input.url)
    .bind(&input.article_uri)
    .bind(input.year)
    .bind(&input.label)
    .bind(input.sort_order)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn delete_version(pool: &PgPool, version_id: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM paper_versions WHERE id = $1")
        .bind(version_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn add_author(
    pool: &PgPool,
    paper_id: &str,
    author_id: &str,
    position: i16,
    role: &str,
) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO paper_authors (paper_id, author_id, position, role) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (paper_id, author_id) DO UPDATE SET position = EXCLUDED.position, role = EXCLUDED.role",
    )
    .bind(paper_id)
    .bind(author_id)
    .bind(position)
    .bind(role)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_paper(pool: &PgPool, id: &str) -> crate::Result<()> {
    sqlx::query("UPDATE papers SET removed_at = now() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Look up a paper that has a `kind='native'` version pointing at the given
/// article URI. Lets the article page show "this article is the canonical
/// text of paper X" with a link upward.
pub async fn paper_for_native_article(pool: &PgPool, article_uri: &str) -> crate::Result<Option<Paper>> {
    let paper = sqlx::query_as::<_, Paper>(
        "SELECT p.id, p.title, p.abstract AS abstract_, p.authors, p.venue, p.venue_kind, \
                p.year, p.doi, p.arxiv_id, p.bibtex_key, p.accepted, p.created_by, p.created_at \
         FROM papers p \
         JOIN paper_versions pv ON pv.paper_id = p.id \
         WHERE pv.kind = 'native' AND pv.article_uri = $1 AND p.removed_at IS NULL \
         LIMIT 1",
    )
    .bind(article_uri)
    .fetch_optional(pool)
    .await?;
    Ok(paper)
}
