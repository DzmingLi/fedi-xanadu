mod admin;
mod appeals;
mod articles;
mod blocks;
mod books;
mod auth;
mod bookmarks;
mod comments;
mod drafts;
mod follows;
mod graph;
mod interests;
mod learned;
mod members;
mod notifications;
mod keybindings;
mod profile;
mod settings;
mod questions;
mod reports;
mod series;
mod skill_trees;
mod skills;
mod tags;
mod votes;

use axum::{
    Json, Router,
    extract::{FromRequestParts, State},
    http::{HeaderMap, request::Parts},
    routing::{get, post},
};
use fx_core::services::auth_service;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    // Stricter rate limit for auth endpoints (5 requests per minute per real client IP)
    let auth_governor_conf = std::sync::Arc::new(
        tower_governor::governor::GovernorConfigBuilder::default()
            .key_extractor(tower_governor::key_extractor::SmartIpKeyExtractor)
            .per_second(12) // 1 token every 12 seconds
            .burst_size(5)  // max 5 burst
            .finish()
            .expect("invalid auth rate limiter config"),
    );
    let auth_limiter = tower_governor::GovernorLayer::new(auth_governor_conf);

    let auth_routes = Router::new()
        .route("/auth/login", post(auth::login))
        .layer(auth_limiter);

    Router::new()
        .merge(auth_routes)
        .route("/health", get(health))
        // Auth (non-login routes don't need stricter limit)
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::auth_me))
        // Tags
        .route("/tags", get(tags::list_tags).post(tags::create_tag))
        .route("/tags/by-id", get(tags::get_tag))
        .route("/tags/search", get(tags::search_tags))
        .route("/tags/names", post(tags::update_tag_names))
        .route("/tags/teach", post(tags::set_teach))
        .route("/tags/teach", post(tags::set_teach))
        // Articles
        .route("/articles", get(articles::list_articles).post(articles::create_article))
        .route("/articles/by-uri", get(articles::get_article))
        .route("/articles/by-uri/content", get(articles::get_article_content))
        .route("/articles/full", get(articles::get_article_full))
        .route("/articles/by-uri/prereqs", get(articles::get_article_prereqs))
        .route("/articles/by-uri/forks", get(articles::get_article_forks))
        .route("/articles/all-teaches", get(articles::get_all_article_teaches))
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
        // Format conversion
        .route("/articles/convert", post(articles::convert_content))
        // Image upload & serving
        .route("/articles/upload-image", post(articles::upload_image))
        .route("/articles/image", get(articles::get_image))
        .route("/articles/update", post(articles::update_article))
        .route("/articles/delete", post(articles::delete_article))
        // Access control (paywall)
        .route("/articles/restricted", post(articles::set_restricted))
        .route("/articles/access/grant", post(articles::grant_access))
        .route("/articles/access/revoke", post(articles::revoke_access))
        .route("/articles/access/list", get(articles::list_access_grants))
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
        .route("/bookmarks/public", get(bookmarks::list_public_bookmarks))
        // Learned marks
        .route("/learned", get(learned::list_learned).post(learned::mark_learned))
        .route("/learned/check", get(learned::is_learned))
        .route("/learned/remove", post(learned::unmark_learned))
        // Interests
        .route("/interests", get(interests::get_interests).post(interests::set_interests))
        // Profile
        .route("/profile", get(profile::get_profile))
        .route("/profile/links", post(profile::update_profile_links))
        // Series
        .route("/series", get(series::list_series).post(series::create_series))
        .route("/series/by-id", get(series::get_series_detail))
        .route("/series/tree", get(series::get_series_tree))
        .route("/series/articles", post(series::add_series_article))
        .route("/series/articles/remove", post(series::remove_series_article))
        .route("/series/articles/reorder", post(series::reorder_articles))
        .route("/series/children/reorder", post(series::reorder_children))
        .route("/series/prereqs", post(series::add_series_prereq))
        .route("/series/prereqs/remove", post(series::remove_series_prereq))
        .route("/series/context", get(series::get_series_context))
        .route("/series/all-articles", get(series::all_series_articles))
        // Notifications
        .route("/notifications", get(notifications::list_notifications))
        .route("/notifications/unread", get(notifications::unread_count))
        .route("/notifications/read", post(notifications::mark_read))
        .route("/notifications/read-all", post(notifications::mark_all_read))
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
        // User settings
        .route("/settings", get(settings::get_settings).post(settings::set_settings))
        // Knowledge graph
        .route("/graph", get(graph::get_graph))
        // Questions & Answers
        .route("/questions", get(questions::list_questions).post(questions::create_question))
        .route("/questions/by-uri", get(questions::get_question))
        .route("/questions/by-did", get(questions::get_questions_by_did))
        .route("/questions/by-tag", get(questions::get_questions_by_tag))
        .route("/questions/answer", post(questions::post_answer))
        .route("/answers/by-did", get(questions::get_answers_by_did))
        // Search
        .route("/search", get(articles::search_articles))
        // Admin
        .route("/admin/platform-users", get(admin::list_platform_users).post(admin::create_platform_user))
        .route("/admin/articles", post(admin::admin_create_article))
        .route("/admin/series", post(admin::admin_create_series))
        .route("/admin/series/articles", post(admin::admin_add_series_article))
        .route("/admin/articles/update", post(admin::admin_update_article))
        .route("/admin/articles/delete", post(admin::admin_delete_article))
        .route("/admin/articles/visibility", post(admin::admin_set_visibility))
        .route("/admin/tags/merge", post(admin::admin_merge_tag))
        .route("/admin/questions/merge", post(admin::admin_merge_questions))
        .route("/admin/ban-user", post(admin::admin_ban_user))
        .route("/admin/unban-user", post(admin::admin_unban_user))
        .route("/admin/banned-users", get(admin::admin_list_banned_users))
        .route("/admin/appeals", get(admin::admin_list_appeals))
        .route("/admin/appeals/resolve", post(admin::admin_resolve_appeal))
        // Appeals (user-facing, Auth not WriteAuth so banned users can appeal)
        .route("/appeals", get(appeals::list_my_appeals).post(appeals::create_appeal))
        // Blocks
        .route("/blocks", get(blocks::list_blocked_users).post(blocks::block_user))
        .route("/blocks/remove", post(blocks::unblock_user))
        .route("/blocks/dids", get(blocks::list_blocked_dids))
        // Reports
        .route("/reports", post(reports::create_report))
        .route("/admin/reports", get(admin::admin_list_reports))
        .route("/admin/reports/resolve", post(admin::admin_resolve_report))
        .route("/admin/credentials/verify", post(admin::admin_verify_credentials))
        .route("/admin/credentials/revoke", post(admin::admin_revoke_credentials))
        .route("/admin/questions", post(admin::admin_create_question))
        .route("/admin/questions/answer", post(admin::admin_post_answer))
        // Members
        .route("/members", get(members::list_members).post(members::add_member))
        .route("/members/remove", post(members::remove_member))
        .route("/members/check", get(members::check_membership))
        // Books
        .route("/books", get(books::list_books).post(books::create_book))
        .route("/books/by-id", get(books::get_book))
        .route("/books/update", post(books::update_book))
        .route("/books/editions", post(books::add_edition))
        .route("/books/history", get(books::get_edit_history))
        .route("/books/rate", post(books::rate_book))
        .route("/books/reading-status", post(books::set_reading_status))
        .route("/books/reading-status/remove", post(books::remove_reading_status))
        .route("/books/chapters", get(books::list_chapters).post(books::create_chapter))
        .route("/books/chapters/delete", post(books::delete_chapter))
        .route("/books/chapters/progress", post(books::set_chapter_progress))
}

// --- Health ---

async fn health(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .map_err(|e| AppError(fx_core::Error::Internal(format!("db health check failed: {e}"))))?;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    })))
}

// --- Shared query types ---

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


// --- Auth extractors ---

/// Authenticated user identity extracted from the session token.
#[derive(Debug, Clone)]
pub(crate) struct AuthUser {
    pub did: String,
    pub token: String,
    pub banned: bool,
    pub phone_verified: bool,
}

/// Requires authentication. Returns 401 if no valid session.
pub(crate) struct Auth(pub AuthUser);

impl FromRequestParts<AppState> for Auth {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        match extract_auth_user(&state.pool, &parts.headers).await {
            Some(user) => Ok(Auth(user)),
            None => Err(AppError(fx_core::Error::Unauthorized)),
        }
    }
}

/// Requires authentication + permission to write.
/// Rejects banned users (403) and, on CN instances, users without phone verification.
pub(crate) struct WriteAuth(pub AuthUser);

impl FromRequestParts<AppState> for WriteAuth {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = extract_auth_user(&state.pool, &parts.headers).await
            .ok_or(AppError(fx_core::Error::Unauthorized))?;

        if user.banned {
            return Err(AppError(fx_core::Error::Forbidden {
                action: "account is banned",
            }));
        }

        if state.instance_mode.requires_phone() && !user.phone_verified {
            return Err(AppError(fx_core::Error::Forbidden {
                action: "phone verification required",
            }));
        }

        Ok(WriteAuth(user))
    }
}

/// Optional authentication. Returns `None` if no valid session.
pub(crate) struct MaybeAuth(pub Option<AuthUser>);

impl FromRequestParts<AppState> for MaybeAuth {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        Ok(MaybeAuth(extract_auth_user(&state.pool, &parts.headers).await))
    }
}

async fn extract_auth_user(pool: &sqlx::PgPool, headers: &HeaderMap) -> Option<AuthUser> {
    let token = extract_bearer_token(headers)?;
    let did = auth_service::get_did_by_token(pool, token).await.ok()??;

    // Fetch ban status and phone verification in one query
    let row: Option<(bool, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "SELECT COALESCE(is_banned, false), phone_verified_at FROM platform_users WHERE did = $1",
    )
    .bind(&did)
    .fetch_optional(pool)
    .await
    .ok()?;

    let (banned, phone_verified) = match row {
        Some((b, pv)) => (b, pv.is_some()),
        None => (false, false), // AT Protocol user without platform_users row
    };

    Some(AuthUser { did, token: token.to_string(), banned, phone_verified })
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    Some(auth.strip_prefix("Bearer ").unwrap_or(auth))
}

/// Get PDS session details for AT Protocol side-effects.
/// Returns `None` for platform-local users (no PDS).
pub(crate) async fn pds_session(
    pool: &sqlx::PgPool,
    token: &str,
) -> Option<auth_service::PdsSession> {
    let session = auth_service::get_session_for_pds(pool, token).await.ok()??;
    if session.pds_url.is_empty() {
        return None;
    }
    Some(session)
}

// --- Shared helpers ---

pub(crate) fn uri_to_node_id(uri: &str) -> String {
    uri.replace('/', "_").replace(':', "_")
}

/// Generate a time-sortable ID using microsecond timestamp in base32.
/// Format: 13 chars of base32-encoded microseconds since epoch.
///
/// Uses an atomic counter to guarantee uniqueness even when multiple TIDs
/// are generated within the same microsecond.
pub(crate) fn tid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static LAST_TID: AtomicU64 = AtomicU64::new(0);

    let micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_micros() as u64;

    // Ensure monotonically increasing: if clock hasn't advanced, increment last value.
    let val = loop {
        let prev = LAST_TID.load(Ordering::Relaxed);
        let next = if micros > prev { micros } else { prev + 1 };
        if LAST_TID
            .compare_exchange(prev, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            break next;
        }
    };

    // AT Protocol TID format: base32-sortable, 13 chars
    let chars = b"234567abcdefghijklmnopqrstuvwxyz";
    let mut out = [0u8; 13];
    let mut v = val;
    for byte in out.iter_mut().rev() {
        *byte = chars[(v & 0x1f) as usize];
        v >>= 5;
    }
    // Safety: all bytes are ASCII from the chars array
    String::from_utf8(out.to_vec()).expect("TID is always valid ASCII")
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

pub(crate) fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Log PDS sync failures without blocking the request.
pub(crate) fn log_pds_error<E: std::fmt::Display>(op: &str, e: E) {
    tracing::warn!("PDS sync failed ({op}): {e}");
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- tid ---

    #[test]
    fn tid_length_is_13() {
        let t = tid();
        assert_eq!(t.len(), 13, "TID should be 13 characters, got: {t}");
    }

    #[test]
    fn tid_is_ascii_base32() {
        let t = tid();
        let valid = b"234567abcdefghijklmnopqrstuvwxyz";
        for ch in t.bytes() {
            assert!(valid.contains(&ch), "unexpected char '{}' in TID", ch as char);
        }
    }

    #[test]
    fn tid_is_sortable() {
        let t1 = tid();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let t2 = tid();
        assert!(t2 > t1, "TIDs should be time-sortable: {t1} vs {t2}");
    }

    #[test]
    fn tid_uniqueness() {
        let tids: Vec<String> = (0..1000).map(|_| tid()).collect();
        let set: std::collections::HashSet<&String> = tids.iter().collect();
        assert_eq!(set.len(), tids.len(), "TIDs should be unique even in a tight loop");
    }

    #[test]
    fn tid_monotonically_increasing() {
        let tids: Vec<String> = (0..100).map(|_| tid()).collect();
        for pair in tids.windows(2) {
            assert!(pair[1] > pair[0], "TIDs should be strictly increasing: {} vs {}", pair[0], pair[1]);
        }
    }

    // --- content_hash ---

    #[test]
    fn content_hash_deterministic() {
        let h1 = content_hash("hello");
        let h2 = content_hash("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn content_hash_differs_for_different_input() {
        assert_ne!(content_hash("hello"), content_hash("world"));
    }

    #[test]
    fn content_hash_is_hex_string() {
        let h = content_hash("test");
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()), "hash should be hex: {h}");
        assert_eq!(h.len(), 64, "blake3 hex hash should be 64 chars");
    }

    // --- gen_session_token ---

    #[test]
    fn session_token_length() {
        let token = gen_session_token();
        assert_eq!(token.len(), 64, "32 random bytes = 64 hex chars");
    }

    #[test]
    fn session_token_is_hex() {
        let token = gen_session_token();
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn session_token_uniqueness() {
        let t1 = gen_session_token();
        let t2 = gen_session_token();
        assert_ne!(t1, t2);
    }

    // --- uri_to_node_id ---

    #[test]
    fn uri_to_node_id_replaces_slashes_and_colons() {
        let node_id = uri_to_node_id("at://did:plc:abc/app.bsky.feed.post/123");
        assert!(!node_id.contains('/'));
        assert!(!node_id.contains(':'));
    }

    // --- now_rfc3339 ---

    #[test]
    fn now_rfc3339_parses() {
        let ts = now_rfc3339();
        chrono::DateTime::parse_from_rfc3339(&ts).expect("should be valid RFC3339");
    }

    // --- extract_bearer_token ---

    #[test]
    fn extract_bearer_token_with_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer mytoken123".parse().unwrap());
        assert_eq!(extract_bearer_token(&headers), Some("mytoken123"));
    }

    #[test]
    fn extract_bearer_token_without_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "rawtoken".parse().unwrap());
        assert_eq!(extract_bearer_token(&headers), Some("rawtoken"));
    }

    #[test]
    fn extract_bearer_token_missing() {
        let headers = HeaderMap::new();
        assert_eq!(extract_bearer_token(&headers), None);
    }
}
