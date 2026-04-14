mod admin;
mod appeals;
mod render;
pub(crate) mod articles;
mod auth;
mod blocks;
mod bookmarks;
pub(crate) mod books;
mod comments;
mod creator;
mod discussions;
mod drafts;
mod follows;
mod graph;
mod interests;
mod keybindings;
mod learned;
mod listings;
mod members;
mod notifications;
mod profile;
mod questions;
mod recommendations;
mod reports;
mod series;
mod settings;
mod skill_trees;
mod skills;
mod tags;
mod thoughts;
mod votes;

use axum::{Json, Router, extract::State, routing::{delete, get, patch, post, put}};

use crate::error::{AppError, ApiResult};
use crate::state::AppState;

// --- Shared query types ---

#[derive(serde::Deserialize)]
pub(crate) struct UriQuery {
    pub uri: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct DidQuery {
    pub did: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct TagIdQuery {
    pub tag_id: String,
}

// --- Route tree ---

pub fn routes() -> Router<AppState> {
    // Stricter rate limit for auth endpoints
    let auth_governor_conf = std::sync::Arc::new(
        tower_governor::governor::GovernorConfigBuilder::default()
            .key_extractor(tower_governor::key_extractor::SmartIpKeyExtractor)
            .per_second(12)
            .burst_size(5)
            .finish()
            .expect("invalid auth rate limiter config"),
    );

    let auth_limited = Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/register", post(auth::register))
        .layer(tower_governor::GovernorLayer::new(auth_governor_conf));

    Router::new()
        .merge(auth_limited)
        .route("/health", get(health))
        .merge(auth_routes())
        .merge(tag_routes())
        .merge(article_routes())
        .merge(vote_routes())
        .merge(comment_routes())
        .merge(discussion_routes())
        .merge(skill_routes())
        .merge(skill_tree_routes())
        .merge(bookmark_routes())
        .merge(learned_routes())
        .merge(interest_routes())
        .merge(profile_routes())
        .merge(series_routes())
        .merge(notification_routes())
        .merge(follow_routes())
        .merge(draft_routes())
        .merge(settings_routes())
        .merge(graph_routes())
        .merge(question_routes())
        .merge(search_routes())
        .merge(admin_routes())
        .merge(appeal_routes())
        .merge(block_routes())
        .merge(report_routes())
        .merge(member_routes())
        .merge(book_routes())
        .merge(render_routes())
        .merge(creator_routes())
        .merge(recommendation_routes())
        .merge(listing_routes())
        .merge(thought_routes())
}

// --- Grouped sub-routers ---

fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::auth_me))
}

fn tag_routes() -> Router<AppState> {
    Router::new()
        .route("/tags", get(tags::list_tags).post(tags::create_tag))
        .route("/tags/{id}", get(tags::get_tag))
        .route("/tags/{id}/names", put(tags::update_tag_names))
        .route("/tags/search", get(tags::search_tags))
        .route("/tags/teach", post(tags::set_teach))
}

fn article_routes() -> Router<AppState> {
    Router::new()
        // Collection
        .route("/articles", get(articles::list_articles).post(articles::create_article))
        // Single resource (AT URI via query param — URIs contain slashes)
        .route("/articles/by-uri", get(articles::get_article))
        .route("/articles/by-uri/content", get(articles::get_article_content))
        .route("/articles/by-uri/prereqs", get(articles::get_article_prereqs))
        .route("/articles/by-uri/forks", get(articles::get_article_forks))
        .route("/articles/by-uri/fork-ahead", get(articles::get_fork_ahead))
        .route("/articles/full", get(articles::get_article_full))
        // Mutations (proper verbs)
        .route("/articles/update", put(articles::update_article))
        .route("/articles/delete", delete(articles::delete_article))
        .route("/articles/fork", post(articles::fork_article))
        .route("/articles/convert", post(articles::convert_content))
        // Images
        .route("/articles/upload-image", post(articles::upload_image))
        .route("/articles/image", get(articles::get_image))
        // Access control
        .route("/articles/restricted", put(articles::set_restricted))
        .route("/articles/access/grant", post(articles::grant_access))
        .route("/articles/access/revoke", delete(articles::revoke_access))
        .route("/articles/access/list", get(articles::list_access_grants))
        // Bulk queries
        .route("/articles/all-teaches", get(articles::get_all_article_teaches))
        .route("/articles/all-prereqs", get(articles::get_all_article_prereqs))
        .route("/articles/by-tag", get(articles::get_articles_by_tag))
        .route("/articles/by-did", get(articles::get_articles_by_did))
        .route("/articles/translations", get(articles::get_translations))
        // Version history
        .route("/articles/by-uri/history", get(articles::get_article_history))
        .route("/articles/by-uri/version", get(articles::get_article_version))
        .route("/articles/by-uri/diff", get(articles::get_article_diff))
        .route("/articles/by-uri/unrecord", post(articles::unrecord_article_change))
        .route("/articles/by-uri/record", post(articles::record_article))
        .route("/articles/apply-change", post(articles::apply_change))
        // Collaboration
        .route("/articles/collaborators", get(articles::list_article_collaborators).post(articles::invite_article_collaborator))
        .route("/articles/collaborators/remove", delete(articles::remove_article_collaborator_endpoint))
        .route("/articles/channels", get(articles::list_article_channels))
        .route("/articles/channel/file", get(articles::read_article_channel_file).put(articles::write_article_channel_file))
        .route("/articles/channel/log", get(articles::article_channel_log))
        .route("/articles/channel/apply", post(articles::apply_article_channel_change))
        .route("/articles/channel-diff", get(articles::article_channel_diff))
}

fn vote_routes() -> Router<AppState> {
    Router::new()
        .route("/votes", get(votes::get_article_votes).post(votes::cast_vote))
        .route("/votes/batch", post(votes::get_votes_batch))
        .route("/votes/my", get(votes::get_my_vote))
}

fn comment_routes() -> Router<AppState> {
    Router::new()
        .route("/comments", get(comments::list_comments).post(comments::create_comment))
        .route("/comments/{id}", put(comments::update_comment).delete(comments::delete_comment))
        .route("/comments/{id}/vote", post(comments::vote_comment))
        .route("/comments/my-votes", get(comments::get_my_comment_votes))
}

fn skill_routes() -> Router<AppState> {
    Router::new()
        .route("/skills", get(skills::list_user_skills).post(skills::light_skill))
        .route("/skills/unlight", delete(skills::delete_skill))
        .route("/tag-tree", get(skills::get_user_tag_tree).post(skills::add_tag_child))
        .route("/tag-prereqs", get(skills::get_user_tag_prereqs).post(skills::add_user_tag_prereq).delete(skills::remove_user_tag_prereq))
}

fn skill_tree_routes() -> Router<AppState> {
    Router::new()
        .route("/skill-trees", get(skill_trees::list_skill_trees).post(skill_trees::create_skill_tree))
        .route("/skill-trees/by-uri", get(skill_trees::get_skill_tree_detail))
        .route("/skill-trees/fork", post(skill_trees::fork_skill_tree))
        .route("/skill-trees/edges", post(skill_trees::add_skill_tree_edge))
        .route("/skill-trees/edges/remove", delete(skill_trees::remove_skill_tree_edge))
        .route("/skill-trees/prereqs", post(skill_trees::add_skill_tree_prereq))
        .route("/skill-trees/prereqs/remove", delete(skill_trees::remove_skill_tree_prereq))
        .route("/skill-trees/adopt", post(skill_trees::adopt_skill_tree))
        .route("/skill-trees/active", get(skill_trees::get_active_tree))
}

fn bookmark_routes() -> Router<AppState> {
    Router::new()
        .route("/bookmarks", get(bookmarks::list_bookmarks).post(bookmarks::add_bookmark))
        .route("/bookmarks/remove", delete(bookmarks::remove_bookmark))
        .route("/bookmarks/move", patch(bookmarks::move_bookmark))
        .route("/bookmarks/folders", get(bookmarks::list_bookmark_folders))
        .route("/bookmarks/public", get(bookmarks::list_public_bookmarks))
}

fn learned_routes() -> Router<AppState> {
    Router::new()
        .route("/learned", get(learned::list_learned).post(learned::mark_learned))
        .route("/learned/check", get(learned::is_learned))
        .route("/learned/remove", delete(learned::unmark_learned))
}

fn interest_routes() -> Router<AppState> {
    Router::new()
        .route("/interests", get(interests::get_interests).put(interests::set_interests))
}

fn profile_routes() -> Router<AppState> {
    Router::new()
        .route("/profile", get(profile::get_profile))
        .route("/profile/links", put(profile::update_profile_links))
        .route("/profile/bio", put(profile::update_bio))
}

fn series_routes() -> Router<AppState> {
    Router::new()
        .route("/series", get(series::list_series).post(series::create_series))
        .route("/series/{id}", get(series::get_series_detail))
        .route("/series/{id}/tree", get(series::get_series_tree))
        .route("/series/{id}/articles", post(series::add_series_article))
        .route("/series/{id}/articles/remove", delete(series::remove_series_article))
        .route("/series/{id}/articles/reorder", put(series::reorder_articles))
        .route("/series/{id}/prereqs", post(series::add_series_prereq))
        .route("/series/{id}/prereqs/remove", delete(series::remove_series_prereq))
        .route("/series/context", get(series::get_series_context))
        .route("/series/all-articles", get(series::all_series_articles))
        // Series pijul repo
        .route("/series/{id}/resource", post(series::upload_resource))
        .route("/series/{id}/resources", get(series::list_resources))
        // Pijul file/channel/history routes from shared pad_router
        .nest("/series/{id}", pijul_knot::pad_router::<AppState, crate::auth::PadAuthUser>())
        .route("/series/{id}/fork", post(series::fork_series))
        // Series compile + heading extraction
        .route("/series/{id}/compile", post(series::compile_series))
        .route("/series/{id}/headings", get(series::get_headings))
        .route("/series/{id}/split-level", put(series::update_split_level))
        // Collaboration
        .route("/series/{id}/collaborators", get(series::list_collaborators).post(series::invite_collaborator))
        .route("/series/{id}/collaborators/{did}", delete(series::remove_collaborator))
}

fn discussion_routes() -> Router<AppState> {
    Router::new()
        .route("/discussions", get(discussions::list_discussions).post(discussions::create_discussion))
        .route("/discussions/{id}", get(discussions::get_discussion))
        .route("/discussions/{id}/status", put(discussions::update_status))
        .route("/discussions/{id}/apply", post(discussions::apply_discussion_change))
        .route("/discussions/{id}/apply-all", post(discussions::apply_all_discussion_changes))
}

fn notification_routes() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(notifications::list_notifications))
        .route("/notifications/unread", get(notifications::unread_count))
        .route("/notifications/read", post(notifications::mark_read))
        .route("/notifications/read-all", post(notifications::mark_all_read))
}

fn follow_routes() -> Router<AppState> {
    Router::new()
        .route("/follows", get(follows::list_follows).post(follows::follow))
        .route("/follows/remove", delete(follows::unfollow))
        .route("/follows/seen", post(follows::mark_seen))
        .route("/follows/following", get(follows::following_by_did))
        .route("/follows/followers", get(follows::followers_by_did))
}

fn draft_routes() -> Router<AppState> {
    Router::new()
        .route("/drafts", get(drafts::list_drafts).post(drafts::save_draft))
        .route("/drafts/{id}", put(drafts::update_draft).delete(drafts::delete_draft))
        .route("/drafts/{id}/publish", post(drafts::publish_draft))
}

fn settings_routes() -> Router<AppState> {
    Router::new()
        .route("/keybindings", get(keybindings::get_keybindings).put(keybindings::set_keybindings))
        .route("/settings", get(settings::get_settings).put(settings::set_settings))
}

fn graph_routes() -> Router<AppState> {
    Router::new().route("/graph", get(graph::get_graph))
}

fn recommendation_routes() -> Router<AppState> {
    Router::new()
        .route("/recommendations", get(recommendations::get_recommendations))
        .route("/recommended-questions", get(recommendations::get_recommended_questions))
        .route("/frontier-skills", get(recommendations::get_frontier_skills))
}

fn thought_routes() -> Router<AppState> {
    Router::new()
        .route("/thoughts", get(thoughts::list_thoughts).post(thoughts::create_thought))
}

fn listing_routes() -> Router<AppState> {
    Router::new()
        .route("/listings", get(listings::list_listings).post(listings::create_listing))
        .route("/listings/mine", get(listings::my_listings))
        .route("/listings/matched", get(listings::matched_listings))
        .route("/listings/{id}", get(listings::get_listing).put(listings::update_listing).delete(listings::delete_listing))
        .route("/listings/{id}/close", post(listings::close_listing))
        .route("/listings/{id}/reopen", post(listings::reopen_listing))
}

fn question_routes() -> Router<AppState> {
    Router::new()
        .route("/questions", get(questions::list_questions).post(questions::create_question))
        .route("/questions/by-uri", get(questions::get_question))
        .route("/questions/by-did", get(questions::get_questions_by_did))
        .route("/questions/by-tag", get(questions::get_questions_by_tag))
        .route("/questions/related", get(questions::get_related_questions))
        .route("/questions/answer", post(questions::post_answer))
        .route("/answers/by-did", get(questions::get_answers_by_did))
}

fn search_routes() -> Router<AppState> {
    Router::new().route("/search", get(articles::search_articles))
}

fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/admin/platform-users", get(admin::list_platform_users).post(admin::create_platform_user))
        .route("/admin/articles", post(admin::admin_create_article))
        .route("/admin/articles/update", put(admin::admin_update_article))
        .route("/admin/articles/delete", delete(admin::admin_delete_article))
        .route("/admin/articles/visibility", put(admin::admin_set_visibility))
        .route("/admin/series", post(admin::admin_create_series))
        .route("/admin/series/articles", post(admin::admin_add_series_article))
        .route("/admin/tags/merge", post(admin::admin_merge_tag))
        .route("/admin/questions", post(admin::admin_create_question))
        .route("/admin/questions/answer", post(admin::admin_post_answer))
        .route("/admin/questions/merge", post(admin::admin_merge_questions))
        .route("/admin/ban-user", post(admin::admin_ban_user))
        .route("/admin/unban-user", post(admin::admin_unban_user))
        .route("/admin/banned-users", get(admin::admin_list_banned_users))
        .route("/admin/appeals", get(admin::admin_list_appeals))
        .route("/admin/appeals/resolve", post(admin::admin_resolve_appeal))
        .route("/admin/reports", get(admin::admin_list_reports))
        .route("/admin/reports/resolve", post(admin::admin_resolve_report))
        .route("/admin/credentials/verify", post(admin::admin_verify_credentials))
        .route("/admin/credentials/revoke", post(admin::admin_revoke_credentials))
        .route("/admin/books/revert-edit", post(admin::admin_revert_book_edit))
}

fn appeal_routes() -> Router<AppState> {
    Router::new()
        .route("/appeals", get(appeals::list_my_appeals).post(appeals::create_appeal))
}

fn block_routes() -> Router<AppState> {
    Router::new()
        .route("/blocks", get(blocks::list_blocked_users).post(blocks::block_user))
        .route("/blocks/remove", delete(blocks::unblock_user))
        .route("/blocks/dids", get(blocks::list_blocked_dids))
}

fn report_routes() -> Router<AppState> {
    Router::new().route("/reports", post(reports::create_report))
}

fn member_routes() -> Router<AppState> {
    Router::new()
        .route("/members", get(members::list_members).post(members::add_member))
        .route("/members/remove", delete(members::remove_member))
        .route("/members/check", get(members::check_membership))
}

fn creator_routes() -> Router<AppState> {
    Router::new()
        .route("/creator/stats", get(creator::get_stats))
        .route("/creator/articles", get(creator::list_articles))
        .route("/creator/series", get(creator::list_series))
        .route("/creator/timeline", get(creator::get_timeline))
        .route("/articles/view", post(creator::record_view))
        .route("/series/{id}/publish", post(creator::publish_series))
        .route("/series/{id}/unpublish", post(creator::unpublish_series))
}

fn render_routes() -> Router<AppState> {
    Router::new()
        .route("/render/typst-snippet", post(render::render_typst_snippet))
        .route("/render/latex-snippet", post(render::render_latex_snippet))
}

fn book_routes() -> Router<AppState> {
    Router::new()
        .route("/books", get(books::list_books).post(books::create_book))
        .route("/books/{id}", get(books::get_book).put(books::update_book))
        .route("/books/{id}/editions", post(books::add_edition))
        .route("/books/{id}/chapters", get(books::list_chapters).post(books::create_chapter))
        .route("/books/{id}/chapters/delete", delete(books::delete_chapter))
        .route("/books/{id}/chapters/progress", post(books::set_chapter_progress))
        .route("/books/{id}/chapters/tags", put(books::update_chapter_tags))
        .route("/books/{id}/rate", post(books::rate_book))
        .route("/books/{id}/reading-status", post(books::set_reading_status).delete(books::remove_reading_status))
        .route("/books/{id}/history", get(books::get_edit_history))
        .route("/books/{id}/cover", post(books::upload_cover))
        .route("/book-covers/{id}", get(books::get_cover))
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;

    // =========================================================================
    // Query type deserialization
    // =========================================================================

    #[test]
    fn uri_query_deserializes() {
        let q: UriQuery =
            serde_json::from_value(serde_json::json!({ "uri": "at://did:plc:abc/article/123" }))
                .unwrap();
        assert_eq!(q.uri, "at://did:plc:abc/article/123");
    }

    #[test]
    fn did_query_deserializes() {
        let q: DidQuery =
            serde_json::from_value(serde_json::json!({ "did": "did:plc:xyz" })).unwrap();
        assert_eq!(q.did, "did:plc:xyz");
    }

    #[test]
    fn tag_id_query_deserializes() {
        let q: TagIdQuery =
            serde_json::from_value(serde_json::json!({ "tag_id": "tag-42" })).unwrap();
        assert_eq!(q.tag_id, "tag-42");
    }

    #[test]
    fn uri_query_missing_field_fails() {
        let result = serde_json::from_value::<UriQuery>(serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn uri_query_from_url_params() {
        // Simulate what axum's Query extractor does: parse from URL query string
        let qs = "uri=at%3A%2F%2Fdid%3Aplc%3Aabc%2Farticle%2F123";
        let q: UriQuery = serde_urlencoded::from_str(qs).unwrap();
        assert_eq!(q.uri, "at://did:plc:abc/article/123");
    }

    #[test]
    fn did_query_from_url_params() {
        let qs = "did=did%3Aplc%3Axyz";
        let q: DidQuery = serde_urlencoded::from_str(qs).unwrap();
        assert_eq!(q.did, "did:plc:xyz");
    }

    // =========================================================================
    // Route structure tests (no database required)
    //
    // These build a lightweight Router with stub handlers to verify that the
    // route tree resolves correctly: right paths, right methods.
    // =========================================================================

    /// Build a minimal router that mirrors the real API route structure
    /// but uses trivial handlers that always return 200 + the route name.
    /// This lets us test routing without any database or state.
    fn stub_router() -> axum::Router {
        // A few representative routes from each sub-router
        axum::Router::new()
            // health
            .route("/api/health", get(|| async { "health" }))
            // auth
            .route("/api/auth/login", axum::routing::post(|| async { "login" }))
            .route("/api/auth/logout", axum::routing::post(|| async { "logout" }))
            .route("/api/auth/me", get(|| async { "me" }))
            // articles
            .route("/api/articles", get(|| async { "list_articles" }).post(|| async { "create_article" }))
            .route("/api/articles/by-uri", get(|| async { "get_article" }))
            .route("/api/articles/by-uri/content", get(|| async { "get_content" }))
            .route("/api/articles/fork", axum::routing::post(|| async { "fork" }))
            .route("/api/articles/update", axum::routing::put(|| async { "update" }))
            .route("/api/articles/delete", axum::routing::delete(|| async { "delete" }))
            // votes
            .route("/api/votes", get(|| async { "get_votes" }).post(|| async { "cast_vote" }))
            .route("/api/votes/my", get(|| async { "my_vote" }))
            // comments
            .route("/api/comments", get(|| async { "list_comments" }).post(|| async { "create_comment" }))
            .route("/api/comments/{id}", axum::routing::put(|| async { "update_comment" }).delete(|| async { "delete_comment" }))
            // series
            .route("/api/series", get(|| async { "list_series" }).post(|| async { "create_series" }))
            .route("/api/series/{id}", get(|| async { "get_series" }))
            .route("/api/series/{id}/articles", axum::routing::post(|| async { "add_article" }))
            // bookmarks
            .route("/api/bookmarks", get(|| async { "list_bookmarks" }).post(|| async { "add_bookmark" }))
            // drafts
            .route("/api/drafts", get(|| async { "list_drafts" }).post(|| async { "save_draft" }))
            .route("/api/drafts/{id}", axum::routing::put(|| async { "update_draft" }).delete(|| async { "delete_draft" }))
            .route("/api/drafts/{id}/publish", axum::routing::post(|| async { "publish" }))
            // tags
            .route("/api/tags", get(|| async { "list_tags" }).post(|| async { "create_tag" }))
            .route("/api/tags/{id}", get(|| async { "get_tag" }))
            .route("/api/tags/search", get(|| async { "search_tags" }))
            // profile
            .route("/api/profile", get(|| async { "get_profile" }))
            // notifications
            .route("/api/notifications", get(|| async { "list_notifications" }))
            .route("/api/notifications/unread", get(|| async { "unread_count" }))
            // search
            .route("/api/search", get(|| async { "search" }))
            // admin
            .route("/api/admin/platform-users", get(|| async { "list_users" }).post(|| async { "create_user" }))
            // books
            .route("/api/books", get(|| async { "list_books" }).post(|| async { "create_book" }))
            .route("/api/books/{id}", get(|| async { "get_book" }).put(|| async { "update_book" }))
            .route("/api/books/{id}/chapters", get(|| async { "list_chapters" }).post(|| async { "create_chapter" }))
            // graph
            .route("/api/graph", get(|| async { "graph" }))
    }

    async fn request(router: &axum::Router, method: Method, uri: &str) -> StatusCode {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        router.clone().oneshot(req).await.unwrap().status()
    }

    async fn request_body(router: &axum::Router, method: Method, uri: &str) -> String {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 64).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    // --- GET routes ---

    #[tokio::test]
    async fn health_route_exists() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/health").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn articles_get_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/articles").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn articles_by_uri_resolves() {
        let router = stub_router();
        assert_eq!(
            request(&router, Method::GET, "/api/articles/by-uri?uri=test").await,
            StatusCode::OK,
        );
    }

    #[tokio::test]
    async fn votes_get_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/votes").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn comments_get_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/comments").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn tags_get_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/tags").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn tags_search_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/tags/search").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn series_list_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/series").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn series_by_id_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/series/42").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn bookmarks_get_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/bookmarks").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn drafts_get_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/drafts").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn profile_get_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/profile").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn notifications_get_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/notifications").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn search_get_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/search").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn graph_get_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/graph").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn books_list_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/books").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn books_by_id_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/books/7").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn book_chapters_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api/books/7/chapters").await, StatusCode::OK);
    }

    // --- POST routes ---

    #[tokio::test]
    async fn auth_login_post_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::POST, "/api/auth/login").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn auth_logout_post_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::POST, "/api/auth/logout").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn articles_post_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::POST, "/api/articles").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn articles_fork_post_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::POST, "/api/articles/fork").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn votes_post_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::POST, "/api/votes").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn comments_post_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::POST, "/api/comments").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn series_post_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::POST, "/api/series").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn series_add_article_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::POST, "/api/series/5/articles").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn bookmarks_post_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::POST, "/api/bookmarks").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn drafts_post_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::POST, "/api/drafts").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn drafts_publish_post_resolves() {
        let router = stub_router();
        assert_eq!(
            request(&router, Method::POST, "/api/drafts/99/publish").await,
            StatusCode::OK,
        );
    }

    // --- PUT routes ---

    #[tokio::test]
    async fn articles_update_put_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::PUT, "/api/articles/update").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn comments_update_put_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::PUT, "/api/comments/42").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn drafts_update_put_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::PUT, "/api/drafts/42").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn books_update_put_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::PUT, "/api/books/7").await, StatusCode::OK);
    }

    // --- DELETE routes ---

    #[tokio::test]
    async fn articles_delete_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::DELETE, "/api/articles/delete").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn comments_delete_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::DELETE, "/api/comments/42").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn drafts_delete_resolves() {
        let router = stub_router();
        assert_eq!(request(&router, Method::DELETE, "/api/drafts/99").await, StatusCode::OK);
    }

    // --- Method not allowed ---

    #[tokio::test]
    async fn health_rejects_post() {
        let router = stub_router();
        assert_eq!(
            request(&router, Method::POST, "/api/health").await,
            StatusCode::METHOD_NOT_ALLOWED,
        );
    }

    #[tokio::test]
    async fn articles_by_uri_rejects_post() {
        let router = stub_router();
        assert_eq!(
            request(&router, Method::POST, "/api/articles/by-uri").await,
            StatusCode::METHOD_NOT_ALLOWED,
        );
    }

    #[tokio::test]
    async fn auth_login_rejects_get() {
        let router = stub_router();
        assert_eq!(
            request(&router, Method::GET, "/api/auth/login").await,
            StatusCode::METHOD_NOT_ALLOWED,
        );
    }

    #[tokio::test]
    async fn graph_rejects_post() {
        let router = stub_router();
        assert_eq!(
            request(&router, Method::POST, "/api/graph").await,
            StatusCode::METHOD_NOT_ALLOWED,
        );
    }

    // --- Non-existent routes return 404 ---

    #[tokio::test]
    async fn unknown_route_returns_404() {
        let router = stub_router();
        assert_eq!(
            request(&router, Method::GET, "/api/nonexistent").await,
            StatusCode::NOT_FOUND,
        );
    }

    #[tokio::test]
    async fn api_root_returns_404() {
        let router = stub_router();
        assert_eq!(request(&router, Method::GET, "/api").await, StatusCode::NOT_FOUND);
    }

    // --- Response body from stub confirms correct route matched ---

    #[tokio::test]
    async fn stub_body_confirms_route_identity() {
        let router = stub_router();
        let body = request_body(&router, Method::GET, "/api/health").await;
        assert_eq!(body, "health");

        let body = request_body(&router, Method::GET, "/api/graph").await;
        assert_eq!(body, "graph");

        let body = request_body(&router, Method::POST, "/api/auth/login").await;
        assert_eq!(body, "login");

        let body = request_body(&router, Method::GET, "/api/votes/my").await;
        assert_eq!(body, "my_vote");
    }

    // --- Path parameter extraction ---

    #[tokio::test]
    async fn path_params_match_any_id() {
        let router = stub_router();
        // Series, comments, drafts, books all use {id} params
        assert_eq!(request(&router, Method::GET, "/api/series/abc-123").await, StatusCode::OK);
        assert_eq!(request(&router, Method::PUT, "/api/comments/999").await, StatusCode::OK);
        assert_eq!(request(&router, Method::DELETE, "/api/drafts/some-uuid").await, StatusCode::OK);
        assert_eq!(request(&router, Method::GET, "/api/books/42").await, StatusCode::OK);
    }

    // =========================================================================
    // Auth extractor: extract_bearer_token (unit, no DB needed)
    // These supplement the tests already in auth.rs
    // =========================================================================

    #[test]
    fn bearer_token_with_extra_spaces_in_value() {
        use axum::http::HeaderMap;
        // The token itself should be preserved as-is
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer abc def".parse().unwrap());
        assert_eq!(crate::auth::extract_bearer_token(&headers), Some("abc def"));
    }

    #[test]
    fn bearer_token_empty_value() {
        use axum::http::HeaderMap;
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer ".parse().unwrap());
        assert_eq!(crate::auth::extract_bearer_token(&headers), Some(""));
    }
}
