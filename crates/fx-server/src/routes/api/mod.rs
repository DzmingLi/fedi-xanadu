mod articles;
mod auth;
mod bookmarks;
mod comments;
mod drafts;
mod follows;
mod graph;
mod interests;
mod keybindings;
mod profile;
mod series;
mod skill_trees;
mod skills;
mod tags;
mod votes;

use axum::{
    Router,
    extract::FromRequestParts,
    http::{HeaderMap, StatusCode, request::Parts},
    routing::{get, post},
};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        // Auth
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::auth_me))
        // Tags
        .route("/tags", get(tags::list_tags).post(tags::create_tag))
        .route("/tags/by-id", get(tags::get_tag))
        // Articles
        .route("/articles", get(articles::list_articles).post(articles::create_article))
        .route("/articles/by-uri", get(articles::get_article))
        .route("/articles/by-uri/content", get(articles::get_article_content))
        .route("/articles/by-uri/prereqs", get(articles::get_article_prereqs))
        .route("/articles/by-uri/forks", get(articles::get_article_forks))
        .route("/articles/all-tags", get(articles::get_all_article_tags))
        .route("/articles/by-tag", get(articles::get_articles_by_tag))
        .route("/articles/all-prereqs", get(articles::get_all_article_prereqs))
        .route("/articles/by-did", get(articles::get_articles_by_did))
        .route("/articles/translations", get(articles::get_translations))
        // Vote
        .route("/vote", post(votes::cast_vote))
        .route("/votes", get(votes::get_article_votes))
        .route("/votes/my", get(votes::get_my_vote))
        // Fork (POST — mutation)
        .route("/articles/fork", post(articles::fork_article))
        // Image upload & serving
        .route("/articles/upload-image", post(articles::upload_image))
        .route("/articles/image", get(articles::get_image))
        .route("/articles/update", post(articles::update_article))
        .route("/articles/delete", post(articles::delete_article))
        // Comments
        .route("/comments", get(comments::list_comments).post(comments::create_comment))
        .route("/comments/update", post(comments::update_comment))
        .route("/comments/delete", post(comments::delete_comment))
        .route("/comments/vote", post(comments::vote_comment))
        .route("/comments/my-votes", get(comments::get_my_comment_votes))
        // User skills
        .route("/skills", get(skills::list_user_skills).post(skills::light_skill))
        .route("/skills/unlight", post(skills::delete_skill))
        // User tag tree (legacy, reads from active skill tree)
        .route("/tag-tree", get(skills::get_user_tag_tree).post(skills::add_tag_child))
        // Skill Trees
        .route("/skill-trees", get(skill_trees::list_skill_trees).post(skill_trees::create_skill_tree))
        .route("/skill-trees/by-uri", get(skill_trees::get_skill_tree_detail))
        .route("/skill-trees/fork", post(skill_trees::fork_skill_tree))
        .route("/skill-trees/edges", post(skill_trees::add_skill_tree_edge))
        .route("/skill-trees/edges/remove", post(skill_trees::remove_skill_tree_edge))
        .route("/skill-trees/adopt", post(skill_trees::adopt_skill_tree))
        .route("/skill-trees/active", get(skill_trees::get_active_tree))
        // Bookmarks
        .route("/bookmarks", get(bookmarks::list_bookmarks).post(bookmarks::add_bookmark))
        .route("/bookmarks/remove", post(bookmarks::remove_bookmark))
        .route("/bookmarks/move", post(bookmarks::move_bookmark))
        .route("/bookmarks/folders", get(bookmarks::list_bookmark_folders))
        // Interests
        .route("/interests", get(interests::get_interests).post(interests::set_interests))
        // Profile
        .route("/profile", get(profile::get_profile))
        .route("/profile/links", post(profile::update_profile_links))
        // Series
        .route("/series", get(series::list_series).post(series::create_series))
        .route("/series/by-id", get(series::get_series_detail))
        .route("/series/articles", post(series::add_series_article))
        .route("/series/articles/remove", post(series::remove_series_article))
        .route("/series/prereqs", post(series::add_series_prereq))
        .route("/series/prereqs/remove", post(series::remove_series_prereq))
        .route("/series/context", get(series::get_series_context))
        .route("/series/all-articles", get(series::all_series_articles))
        // Follows
        .route("/follows", get(follows::list_follows).post(follows::follow))
        .route("/follows/remove", post(follows::unfollow))
        .route("/follows/seen", post(follows::mark_seen))
        .route("/follows/following", get(follows::following_by_did))
        .route("/follows/followers", get(follows::followers_by_did))
        // Drafts
        .route("/drafts", get(drafts::list_drafts).post(drafts::save_draft))
        .route("/drafts/update", post(drafts::update_draft))
        .route("/drafts/delete", post(drafts::delete_draft))
        .route("/drafts/publish", post(drafts::publish_draft))
        // Keybindings
        .route("/keybindings", get(keybindings::get_keybindings).post(keybindings::set_keybindings))
        // Knowledge graph
        .route("/graph", get(graph::get_graph))
}

// --- Health ---

async fn health() -> &'static str {
    "ok"
}

// --- Shared types ---

#[derive(serde::Deserialize)]
pub(crate) struct UriQuery {
    pub uri: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct IdQuery {
    pub id: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct DidQuery {
    pub did: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct TagIdQuery {
    pub tag_id: String,
}

// --- AuthDid extractor ---

/// Extracts the current user's DID from the Authorization header.
/// Falls back to "did:plc:anonymous" if no valid session.
pub(crate) struct AuthDid(pub String);

impl FromRequestParts<AppState> for AuthDid {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let did = current_did_from_headers(&state.pool, &parts.headers).await;
        Ok(AuthDid(did))
    }
}

/// Extracts the current user's DID, returning UNAUTHORIZED if not logged in.
pub(crate) struct RequireAuth(pub String);

impl FromRequestParts<AppState> for RequireAuth {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let did = current_did_from_headers(&state.pool, &parts.headers).await;
        if did == "did:plc:anonymous" {
            Err(StatusCode::UNAUTHORIZED)
        } else {
            Ok(RequireAuth(did))
        }
    }
}

// --- Shared helpers ---

async fn current_did_from_headers(pool: &sqlx::SqlitePool, headers: &HeaderMap) -> String {
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        let token = auth.strip_prefix("Bearer ").unwrap_or(auth);
        if let Ok(Some(did)) = sqlx::query_scalar::<_, String>(
            "SELECT did FROM sessions WHERE token = ? AND expires_at > datetime('now')",
        )
        .bind(token)
        .fetch_optional(pool)
        .await
        {
            return did;
        }
    }
    "did:plc:anonymous".to_string()
}

pub(crate) async fn session_from_headers(
    pool: &sqlx::SqlitePool,
    headers: &HeaderMap,
) -> Option<(String, String, String)> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    let token = auth.strip_prefix("Bearer ").unwrap_or(auth);

    #[derive(sqlx::FromRow)]
    struct SessionRow {
        did: String,
        pds_url: String,
        access_jwt: String,
    }

    sqlx::query_as::<_, SessionRow>(
        "SELECT did, pds_url, access_jwt FROM sessions WHERE token = ? AND expires_at > datetime('now')",
    )
    .bind(token)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|s| (s.did, s.pds_url, s.access_jwt))
}

pub(crate) fn uri_to_node_id(uri: &str) -> String {
    uri.replace('/', "_").replace(':', "_")
}

pub(crate) fn tid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{:x}", now)
}

pub(crate) fn content_hash(data: &str) -> String {
    blake3::hash(data.as_bytes()).to_hex().to_string()
}

pub(crate) fn gen_session_token() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let bytes: [u8; 32] = rng.random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let days = secs / 86400;
    let rem = secs % 86400;
    let hours = rem / 3600;
    let minutes = (rem % 3600) / 60;
    let seconds = rem % 60;
    let (year, month, day) = epoch_days_to_date(days as i64);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn epoch_days_to_date(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

pub(crate) const ARTICLE_SELECT: &str = "\
    SELECT a.at_uri, a.did, p.handle AS author_handle, a.title, a.description, \
    a.content_hash, a.content_format, a.lang, a.translation_group, a.license, a.prereq_threshold, \
    COALESCE((SELECT SUM(value) FROM votes WHERE target_uri = a.at_uri), 0) AS vote_score, \
    COALESCE((SELECT COUNT(*) FROM user_bookmarks WHERE article_uri = a.at_uri), 0) AS bookmark_count, \
    a.created_at, a.updated_at \
    FROM articles a LEFT JOIN profiles p ON a.did = p.did";
