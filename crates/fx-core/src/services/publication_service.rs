//! Medium-style content channels (专栏).
//!
//! A publication is a container for articles and series, with multi-editor
//! support via a bilateral handshake: the owner lists members in the
//! publication's `members[]` array, and each member confirms by creating a
//! `publication.membership` record on their own PDS. Only when both sides
//! agree does `membership_at_uri` get set in the database.
//!
//! Content attachment is the **content author's** action: the author creates
//! a `publication.entry` record referencing their own article/series URI
//! and the target publication's URI. This preserves author agency over where
//! their content appears.
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::Result;

/// Locale → text map, reused across profile i18n.
pub type L = HashMap<String, String>;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Publication {
    pub id: String,
    #[ts(type = "Record<string, string>")]
    pub title_i18n: sqlx::types::Json<L>,
    #[ts(type = "Record<string, string>")]
    pub description_i18n: sqlx::types::Json<L>,
    pub cover_url: Option<String>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub at_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PublicationMember {
    pub publication_id: String,
    pub did: String,
    pub role: String,
    pub added_at: chrono::DateTime<chrono::Utc>,
    pub added_by: String,
    pub membership_at_uri: Option<String>,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Member role — uses plain strings in the DB with a CHECK constraint so we
/// can grow the set later without migrations. Order matters for permission
/// checks: owner > editor > writer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Owner,
    Editor,
    Writer,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Editor => "editor",
            Role::Writer => "writer",
        }
    }
    pub fn can_manage_members(&self) -> bool {
        matches!(self, Role::Owner)
    }
    pub fn can_edit_settings(&self) -> bool {
        matches!(self, Role::Owner | Role::Editor)
    }
    pub fn can_publish(&self) -> bool {
        true
    }
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "owner" => Some(Role::Owner),
            "editor" => Some(Role::Editor),
            "writer" => Some(Role::Writer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PublicationEntry {
    pub publication_id: String,
    pub content_uri: String,
    pub content_kind: String,
    pub added_by: String,
    pub added_at: chrono::DateTime<chrono::Utc>,
    pub entry_at_uri: Option<String>,
}

/// One content entry in a publication — either an article (identified by
/// at_uri) or a series (identified by id). Rendered as a single mixed feed
/// on the publication page.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PublicationContentItem {
    pub kind: String,
    pub added_at: chrono::DateTime<chrono::Utc>,
    pub article: Option<crate::models::Article>,
    pub series: Option<crate::services::series_service::SeriesListRow>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PublicationViewerState {
    pub role: Option<String>,
    pub is_following: bool,
    pub membership_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PublicationSummary {
    pub id: String,
    #[ts(type = "Record<string, string>")]
    pub title_i18n: sqlx::types::Json<L>,
    #[ts(type = "Record<string, string>")]
    pub description_i18n: sqlx::types::Json<L>,
    pub cover_url: Option<String>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub member_count: i64,
    pub content_count: i64,
    pub follower_count: i64,
}

// ---- CRUD ----

/// Create a new publication with the caller as owner. Returns the fresh row.
pub async fn create_publication(
    pool: &PgPool,
    owner_did: &str,
    slug: &str,
    title: &L,
    description: &L,
    cover_url: Option<&str>,
) -> Result<Publication> {
    validate_slug(slug)?;

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO publications (id, title_i18n, description_i18n, cover_url, created_by) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(slug)
    .bind(sqlx::types::Json(title))
    .bind(sqlx::types::Json(description))
    .bind(cover_url)
    .bind(owner_did)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO publication_members (publication_id, did, role, added_by) \
         VALUES ($1, $2, 'owner', $2)",
    )
    .bind(slug)
    .bind(owner_did)
    .execute(&mut *tx)
    .await?;

    let pub_ = sqlx::query_as::<_, Publication>(
        "SELECT * FROM publications WHERE id = $1",
    )
    .bind(slug)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(pub_)
}

pub async fn get_publication(pool: &PgPool, slug: &str) -> Result<Option<Publication>> {
    Ok(sqlx::query_as::<_, Publication>("SELECT * FROM publications WHERE id = $1")
        .bind(slug)
        .fetch_optional(pool)
        .await?)
}

pub async fn list_publications(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<PublicationSummary>> {
    Ok(sqlx::query_as::<_, PublicationSummary>(
        "SELECT p.id, p.title_i18n, p.description_i18n, p.cover_url, p.created_by, p.created_at, \
            (SELECT COUNT(*) FROM publication_members WHERE publication_id = p.id) AS member_count, \
            (SELECT COUNT(*) FROM publication_content WHERE publication_id = p.id) AS content_count, \
            (SELECT COUNT(*) FROM publication_followers WHERE publication_id = p.id) AS follower_count \
         FROM publications p \
         ORDER BY p.created_at DESC \
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?)
}

pub async fn update_publication(
    pool: &PgPool,
    caller_did: &str,
    slug: &str,
    title: &L,
    description: &L,
    cover_url: Option<&str>,
) -> Result<()> {
    require_role(pool, slug, caller_did, &[Role::Owner, Role::Editor]).await?;
    sqlx::query(
        "UPDATE publications \
         SET title_i18n = $1, description_i18n = $2, cover_url = $3, updated_at = NOW() \
         WHERE id = $4",
    )
    .bind(sqlx::types::Json(title))
    .bind(sqlx::types::Json(description))
    .bind(cover_url)
    .bind(slug)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_publication(pool: &PgPool, caller_did: &str, slug: &str) -> Result<()> {
    require_role(pool, slug, caller_did, &[Role::Owner]).await?;
    sqlx::query("DELETE FROM publications WHERE id = $1")
        .bind(slug)
        .execute(pool)
        .await?;
    Ok(())
}

/// Persist the AT Proto record URI for the publication itself. Called once
/// the PDS round-trip succeeds.
pub async fn set_publication_at_uri(pool: &PgPool, slug: &str, at_uri: &str) -> Result<()> {
    sqlx::query("UPDATE publications SET at_uri = $1 WHERE id = $2")
        .bind(at_uri)
        .bind(slug)
        .execute(pool)
        .await?;
    Ok(())
}

// ---- Members ----

pub async fn list_members(pool: &PgPool, slug: &str) -> Result<Vec<PublicationMember>> {
    Ok(sqlx::query_as::<_, PublicationMember>(
        "SELECT m.publication_id, m.did, m.role, m.added_at, m.added_by, m.membership_at_uri, \
                p.handle, p.display_name, p.avatar_url \
         FROM publication_members m \
         LEFT JOIN profiles p ON p.did = m.did \
         WHERE m.publication_id = $1 \
         ORDER BY CASE m.role WHEN 'owner' THEN 0 WHEN 'editor' THEN 1 ELSE 2 END, m.added_at",
    )
    .bind(slug)
    .fetch_all(pool)
    .await?)
}

pub async fn add_member(
    pool: &PgPool,
    caller_did: &str,
    slug: &str,
    did: &str,
    role: Role,
) -> Result<()> {
    if matches!(role, Role::Owner) {
        return Err(crate::Error::BadRequest("cannot invite another owner".into()));
    }
    require_role(pool, slug, caller_did, &[Role::Owner]).await?;
    sqlx::query(
        "INSERT INTO publication_members (publication_id, did, role, added_by) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (publication_id, did) DO UPDATE SET role = EXCLUDED.role",
    )
    .bind(slug)
    .bind(did)
    .bind(role.as_str())
    .bind(caller_did)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_member(
    pool: &PgPool,
    caller_did: &str,
    slug: &str,
    did: &str,
) -> Result<()> {
    require_role(pool, slug, caller_did, &[Role::Owner]).await?;
    if caller_did == did {
        return Err(crate::Error::BadRequest(
            "owner cannot remove themselves; transfer ownership first".into(),
        ));
    }
    sqlx::query("DELETE FROM publication_members WHERE publication_id = $1 AND did = $2 AND role <> 'owner'")
        .bind(slug)
        .bind(did)
        .execute(pool)
        .await?;
    Ok(())
}

/// Record that a member has accepted (their PDS membership record URI is now
/// known). Idempotent.
pub async fn confirm_membership(
    pool: &PgPool,
    slug: &str,
    did: &str,
    membership_at_uri: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE publication_members SET membership_at_uri = $1 \
         WHERE publication_id = $2 AND did = $3",
    )
    .bind(membership_at_uri)
    .bind(slug)
    .bind(did)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn leave_publication(pool: &PgPool, did: &str, slug: &str) -> Result<()> {
    sqlx::query(
        "DELETE FROM publication_members \
         WHERE publication_id = $1 AND did = $2 AND role <> 'owner'",
    )
    .bind(slug)
    .bind(did)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn user_role(
    pool: &PgPool,
    slug: &str,
    did: &str,
) -> Result<Option<Role>> {
    let r: Option<String> = sqlx::query_scalar(
        "SELECT role FROM publication_members WHERE publication_id = $1 AND did = $2",
    )
    .bind(slug)
    .bind(did)
    .fetch_optional(pool)
    .await?;
    Ok(r.and_then(|s| Role::parse(&s)))
}

async fn require_role(
    pool: &PgPool,
    slug: &str,
    did: &str,
    allowed: &[Role],
) -> Result<()> {
    let role = user_role(pool, slug, did).await?;
    match role {
        Some(r) if allowed.contains(&r) => Ok(()),
        _ => Err(crate::Error::BadRequest("insufficient publication role".into())),
    }
}

// ---- Content (cross-post) ----

/// Cross-post an article or series to a publication. Caller must be the
/// content author AND have at least writer role on the publication.
pub async fn add_content(
    pool: &PgPool,
    caller_did: &str,
    slug: &str,
    content_uri: &str,
    content_kind: &str,
) -> Result<()> {
    if !matches!(content_kind, "article" | "series") {
        return Err(crate::Error::BadRequest("invalid content_kind".into()));
    }
    require_role(pool, slug, caller_did, &[Role::Owner, Role::Editor, Role::Writer]).await?;

    // `content_uri` holds different identifiers per kind: articles have a
    // proper at_uri, series are identified by their short `id`.
    let author: Option<String> = match content_kind {
        "article" => sqlx::query_scalar(
            "SELECT a.author_did FROM articles a \
             JOIN article_localizations l \
                 ON l.repo_uri = a.repo_uri AND l.source_path = a.source_path \
             WHERE l.at_uri = $1",
        )
            .bind(content_uri)
            .fetch_optional(pool)
            .await?,
        _ => sqlx::query_scalar("SELECT created_by FROM series WHERE id = $1")
            .bind(content_uri)
            .fetch_optional(pool)
            .await?,
    };
    let Some(author_did) = author else {
        return Err(crate::Error::BadRequest("content not found".into()));
    };
    if author_did != caller_did {
        return Err(crate::Error::BadRequest(
            "only the content author can attach their content to a publication".into(),
        ));
    }

    sqlx::query(
        "INSERT INTO publication_content (publication_id, content_uri, content_kind, added_by) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT DO NOTHING",
    )
    .bind(slug)
    .bind(content_uri)
    .bind(content_kind)
    .bind(caller_did)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_content(
    pool: &PgPool,
    caller_did: &str,
    slug: &str,
    content_uri: &str,
) -> Result<()> {
    // Author can always withdraw their own; editors/owner can also remove.
    let added_by: Option<String> = sqlx::query_scalar(
        "SELECT added_by FROM publication_content WHERE publication_id = $1 AND content_uri = $2",
    )
    .bind(slug)
    .bind(content_uri)
    .fetch_optional(pool)
    .await?;

    let is_author = added_by.as_deref() == Some(caller_did);
    let can_curate = user_role(pool, slug, caller_did)
        .await?
        .map(|r| matches!(r, Role::Owner | Role::Editor))
        .unwrap_or(false);
    if !is_author && !can_curate {
        return Err(crate::Error::BadRequest("not authorized to remove this entry".into()));
    }

    sqlx::query("DELETE FROM publication_content WHERE publication_id = $1 AND content_uri = $2")
        .bind(slug)
        .bind(content_uri)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_entry_at_uri(
    pool: &PgPool,
    slug: &str,
    content_uri: &str,
    entry_at_uri: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE publication_content SET entry_at_uri = $1 \
         WHERE publication_id = $2 AND content_uri = $3",
    )
    .bind(entry_at_uri)
    .bind(slug)
    .bind(content_uri)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_content_uris(
    pool: &PgPool,
    slug: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<(String, String, chrono::DateTime<chrono::Utc>)>> {
    let rows: Vec<(String, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT content_uri, content_kind, added_at FROM publication_content \
         WHERE publication_id = $1 \
         ORDER BY added_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(slug)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// All publications that a given content URI is attached to — used to show
/// cross-post badges on article/series detail pages.
pub async fn publications_for_content(
    pool: &PgPool,
    content_uri: &str,
) -> Result<Vec<Publication>> {
    Ok(sqlx::query_as::<_, Publication>(
        "SELECT p.* FROM publications p \
         JOIN publication_content c ON c.publication_id = p.id \
         WHERE c.content_uri = $1",
    )
    .bind(content_uri)
    .fetch_all(pool)
    .await?)
}

// ---- Follow ----

pub async fn follow(pool: &PgPool, did: &str, slug: &str, follow_at_uri: Option<&str>) -> Result<()> {
    sqlx::query(
        "INSERT INTO publication_followers (publication_id, did, follow_at_uri) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (publication_id, did) DO UPDATE SET follow_at_uri = COALESCE(EXCLUDED.follow_at_uri, publication_followers.follow_at_uri)",
    )
    .bind(slug)
    .bind(did)
    .bind(follow_at_uri)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn unfollow(pool: &PgPool, did: &str, slug: &str) -> Result<()> {
    sqlx::query("DELETE FROM publication_followers WHERE publication_id = $1 AND did = $2")
        .bind(slug)
        .bind(did)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn is_following(pool: &PgPool, did: &str, slug: &str) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM publication_followers WHERE publication_id = $1 AND did = $2",
    )
    .bind(slug)
    .bind(did)
    .fetch_one(pool)
    .await?
        > 0)
}

/// Slugs of publications the user follows — used by the home feed.
pub async fn followed_publication_ids(pool: &PgPool, did: &str) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT publication_id FROM publication_followers WHERE did = $1",
    )
    .bind(did)
    .fetch_all(pool)
    .await?)
}

/// Publications a user is a confirmed member of — for the "cross-post to"
/// dropdown on the publish form.
pub async fn writable_publications_for(
    pool: &PgPool,
    did: &str,
) -> Result<Vec<Publication>> {
    Ok(sqlx::query_as::<_, Publication>(
        "SELECT p.* FROM publications p \
         JOIN publication_members m ON m.publication_id = p.id \
         WHERE m.did = $1 \
         ORDER BY p.created_at DESC",
    )
    .bind(did)
    .fetch_all(pool)
    .await?)
}

// ---- Validation ----

fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty() || slug.len() > 64 {
        return Err(crate::Error::BadRequest("slug must be 1-64 chars".into()));
    }
    if !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(crate::Error::BadRequest("slug must be [a-z0-9_-]".into()));
    }
    Ok(())
}
