use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct FollowedUser {
    pub follows_did: String,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub has_update: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct FollowEntry {
    pub did: String,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Fixed set of contact fields shown on the profile card, plus an optional
/// list of free-form extra links. Omitted keys should be stripped by the
/// frontend so the stored JSON stays compact.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Contacts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telegram: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codeberg: Option<String>,
    /// Tangled stores both a full URL (for the hyperlink) and a username
    /// (for display), since usernames don't map to a canonical URL pattern.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tangled: Option<LinkedHandle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub youtube: Option<String>,
    /// Bilibili — same rationale as tangled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bilibili: Option<LinkedHandle>,
    /// Free-form extra links with user-supplied labels.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom: Vec<CustomLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CustomLink {
    pub label: String,
    pub url: String,
}

/// A hyperlink whose displayed text and destination URL differ, used where
/// platforms (bilibili, tangled) don't have a canonical `domain/{username}`
/// URL pattern.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct LinkedHandle {
    pub url: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct EducationEntry {
    pub degree: String,
    pub school: String,
    #[serde(default)]
    pub department: Option<String>,
    #[serde(default)]
    pub major: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    /// true = currently enrolled / in progress
    #[serde(default)]
    pub current: bool,
    /// Locale → translated text fields (school, department, major).
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    #[ts(type = "Record<string, EducationTranslation>")]
    pub translations: std::collections::HashMap<String, EducationTranslation>,
}

/// Translated text fields for an education entry.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct EducationTranslation {
    #[serde(default)]
    pub school: Option<String>,
    #[serde(default)]
    pub department: Option<String>,
    #[serde(default)]
    pub major: Option<String>,
}

/// Locale -> text map. E.g. `{"en": "NightBoat", "zh": "夜舟"}`.
pub type L = std::collections::HashMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct PublicationEntry {
    pub title: L,
    pub authors: Vec<String>,
    #[serde(default)]
    pub venue: L,
    #[serde(default)]
    pub year: i32,
    pub url: Option<String>,
    pub doi: Option<String>,
    #[serde(default, rename = "abstract")]
    pub abstract_text: Option<L>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ProjectEntry {
    pub title: L,
    #[serde(default)]
    pub description: L,
    pub url: Option<String>,
    #[serde(default = "default_active")]
    pub status: String,
}

fn default_active() -> String { "active".into() }

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct TeachingEntry {
    pub course_name: L,
    #[serde(default)]
    pub role: L,
    #[serde(default)]
    pub institution: L,
    #[serde(default)]
    pub year: i32,
    pub description: Option<L>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ProfileResponse {
    pub did: String,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub bio: String,
    pub reputation: i32,
    pub article_count: i64,
    pub series_count: i64,
    pub contacts: Contacts,
    pub email: Option<String>,
    pub education: Vec<EducationEntry>,
    pub publications: Vec<PublicationEntry>,
    pub projects: Vec<ProjectEntry>,
    pub teaching: Vec<TeachingEntry>,
    pub affiliation: Option<String>,
    pub credentials_verified: bool,
}

pub async fn list_follows(pool: &PgPool, did: &str) -> crate::Result<Vec<FollowedUser>> {
    let rows = sqlx::query_as::<_, FollowedUser>(
        "SELECT f.follows_did, p.handle, p.display_name, p.avatar_url, \
         EXISTS ( \
           SELECT 1 FROM articles a WHERE a.did = f.follows_did \
           AND a.created_at > COALESCE( \
             (SELECT last_seen_at FROM follow_seen WHERE did = f.did AND follows_did = f.follows_did), \
             f.created_at \
           ) \
         ) AS has_update \
         FROM user_follows f \
         LEFT JOIN profiles p ON f.follows_did = p.did \
         WHERE f.did = $1 \
         ORDER BY f.created_at DESC",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn follow(pool: &PgPool, did: &str, follows_did: &str) -> crate::Result<()> {
    if did == follows_did {
        return Err(crate::Error::BadRequest("cannot follow yourself".into()));
    }
    sqlx::query(
        "INSERT INTO user_follows (did, follows_did) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(did)
    .bind(follows_did)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn unfollow(pool: &PgPool, did: &str, follows_did: &str) -> crate::Result<()> {
    sqlx::query("DELETE FROM user_follows WHERE did = $1 AND follows_did = $2")
        .bind(did)
        .bind(follows_did)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn mark_seen(pool: &PgPool, did: &str, follows_did: &str) -> crate::Result<()> {
    sqlx::query(
        "INSERT INTO follow_seen (did, follows_did, last_seen_at) VALUES ($1, $2, NOW()) \
         ON CONFLICT(did, follows_did) DO UPDATE SET last_seen_at = NOW()",
    )
    .bind(did)
    .bind(follows_did)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn following_by_did(pool: &PgPool, did: &str) -> crate::Result<Vec<FollowEntry>> {
    let rows = sqlx::query_as::<_, FollowEntry>(
        "SELECT f.follows_did AS did, p.handle, p.display_name, p.avatar_url \
         FROM user_follows f \
         LEFT JOIN profiles p ON f.follows_did = p.did \
         WHERE f.did = $1 \
         ORDER BY f.created_at DESC",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn followers_by_did(pool: &PgPool, did: &str) -> crate::Result<Vec<FollowEntry>> {
    let rows = sqlx::query_as::<_, FollowEntry>(
        "SELECT f.did AS did, p.handle, p.display_name, p.avatar_url \
         FROM user_follows f \
         LEFT JOIN profiles p ON f.did = p.did \
         WHERE f.follows_did = $1 \
         ORDER BY f.created_at DESC",
    )
    .bind(did)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_profile(pool: &PgPool, did: &str) -> crate::Result<ProfileResponse> {
    #[derive(sqlx::FromRow)]
    struct ProfileRow {
        handle: Option<String>,
        display_name: Option<String>,
        avatar_url: Option<String>,
        banner_url: Option<String>,
        bio: String,
        reputation: i32,
        contacts: serde_json::Value,
        email: Option<String>,
        education: serde_json::Value,
        publications: serde_json::Value,
        projects: serde_json::Value,
        teaching: serde_json::Value,
        affiliation: Option<String>,
        credentials_verified: Option<bool>,
        article_count: i64,
        series_count: i64,
    }

    let row = sqlx::query_as::<_, ProfileRow>(
        "SELECT \
            p.handle, p.display_name, p.avatar_url, p.banner_url, p.bio, p.reputation, p.contacts, \
            us.email, \
            p.education, p.publications, p.projects, p.teaching, \
            p.affiliation, \
            p.credentials_verified, \
            (SELECT COUNT(*) FROM articles WHERE did = $1) AS article_count, \
            (SELECT COUNT(*) FROM series WHERE created_by = $1) AS series_count \
         FROM profiles p \
         LEFT JOIN user_settings us ON us.did = p.did \
         WHERE p.did = $1",
    )
    .bind(did)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let contacts: Contacts =
                serde_json::from_value(r.contacts).unwrap_or_default();
            let education: Vec<EducationEntry> =
                serde_json::from_value(r.education).unwrap_or_default();
            let publications: Vec<PublicationEntry> =
                serde_json::from_value(r.publications).unwrap_or_default();
            let projects: Vec<ProjectEntry> =
                serde_json::from_value(r.projects).unwrap_or_default();
            let teaching: Vec<TeachingEntry> =
                serde_json::from_value(r.teaching).unwrap_or_default();
            Ok(ProfileResponse {
                did: did.to_string(),
                handle: r.handle,
                display_name: r.display_name,
                avatar_url: r.avatar_url,
                banner_url: r.banner_url,
                bio: r.bio,
                reputation: r.reputation,
                article_count: r.article_count,
                series_count: r.series_count,
                contacts,
                email: r.email,
                education,
                publications,
                projects,
                teaching,
                affiliation: r.affiliation,
                credentials_verified: r.credentials_verified.unwrap_or(false),
            })
        }
        None => Ok(ProfileResponse {
            did: did.to_string(),
            handle: None,
            display_name: None,
            avatar_url: None,
            banner_url: None,
            bio: String::new(),
            reputation: 0,
            article_count: 0,
            series_count: 0,
            contacts: Contacts::default(),
            email: None,
            education: Vec::new(),
            publications: Vec::new(),
            projects: Vec::new(),
            teaching: Vec::new(),
            affiliation: None,
            credentials_verified: false,
        }),
    }
}

pub async fn update_profile_contacts(
    pool: &PgPool,
    did: &str,
    contacts: &Contacts,
) -> crate::Result<()> {
    sqlx::query("UPDATE profiles SET contacts = $1 WHERE did = $2")
        .bind(sqlx::types::Json(contacts))
        .bind(did)
        .execute(pool)
        .await?;
    Ok(())
}
