use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FollowedUser {
    pub follows_did: String,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub has_update: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FollowEntry {
    pub did: String,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EducationEntry {
    pub degree: String,
    pub school: String,
    #[serde(default)]
    pub year: String,
    /// true = currently enrolled / in progress
    #[serde(default)]
    pub current: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileResponse {
    pub did: String,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub article_count: i64,
    pub series_count: i64,
    pub links: Vec<ProfileLink>,
    pub email: Option<String>,
    pub education: Vec<EducationEntry>,
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
        links: Option<String>,
        email: Option<String>,
        education: serde_json::Value,
        affiliation: Option<String>,
        credentials_verified: Option<bool>,
        article_count: i64,
        series_count: i64,
    }

    let row = sqlx::query_as::<_, ProfileRow>(
        "SELECT \
            p.handle, p.display_name, p.avatar_url, p.links, \
            us.email, \
            p.education, \
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

    let links: Vec<ProfileLink> = row
        .as_ref()
        .and_then(|r| r.links.as_deref())
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or_default();

    match row {
        Some(r) => {
            let education: Vec<EducationEntry> =
                serde_json::from_value(r.education).unwrap_or_default();
            Ok(ProfileResponse {
                did: did.to_string(),
                handle: r.handle,
                display_name: r.display_name,
                avatar_url: r.avatar_url,
                article_count: r.article_count,
                series_count: r.series_count,
                links,
                email: r.email,
                education,
                affiliation: r.affiliation,
                credentials_verified: r.credentials_verified.unwrap_or(false),
            })
        }
        None => Ok(ProfileResponse {
            did: did.to_string(),
            handle: None,
            display_name: None,
            avatar_url: None,
            article_count: 0,
            series_count: 0,
            links: Vec::new(),
            email: None,
            education: Vec::new(),
            affiliation: None,
            credentials_verified: false,
        }),
    }
}

pub async fn update_profile_links(
    pool: &PgPool,
    did: &str,
    links_json: &str,
) -> crate::Result<()> {
    sqlx::query("UPDATE profiles SET links = $1 WHERE did = $2")
        .bind(links_json)
        .bind(did)
        .execute(pool)
        .await?;
    Ok(())
}
