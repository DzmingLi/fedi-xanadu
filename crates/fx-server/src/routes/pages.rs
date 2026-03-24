use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Html,
    routing::get,
};

#[derive(serde::Deserialize)]
struct UriQuery {
    uri: String,
}

use crate::state::AppState;
use crate::templates;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/tags", get(tags_page))
        .route("/article", get(article_page))
        .route("/new", get(new_article_page))
        .route("/skills", get(skills_page))
        .route("/graph", get(graph_page))
}

async fn index(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let articles = sqlx::query_as::<_, fx_core::models::Article>(
        "SELECT * FROM articles ORDER BY created_at DESC LIMIT 20",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(templates::index_page(
        &state.config.instance_name,
        &articles,
    )))
}

async fn tags_page(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let tags = sqlx::query_as::<_, fx_core::models::Tag>("SELECT * FROM tags ORDER BY name")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(templates::tags_page(
        &state.config.instance_name,
        &tags,
    )))
}

async fn article_page(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> Result<Html<String>, StatusCode> {
    let article = sqlx::query_as::<_, fx_core::models::Article>(
        "SELECT * FROM articles WHERE at_uri = ?",
    )
    .bind(&uri)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Read content
    let node_id = uri.replace('/', "_").replace(':', "_");
    let content_path = state.pijul.repo_path(&node_id).join("content.typ");
    let source = std::fs::read_to_string(&content_path).unwrap_or_default();
    let html = fx_render::render_typst_to_html(&source).unwrap_or_default();

    // Get prereqs
    let prereqs = sqlx::query_as::<_, fx_core::models::ArticlePrereqRow>(
        "SELECT ap.tag_id, ap.prereq_type, t.name as tag_name
         FROM article_prereqs ap
         JOIN tags t ON t.id = ap.tag_id
         WHERE ap.article_uri = ?",
    )
    .bind(&uri)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Get forks
    let forks = sqlx::query_as::<_, fx_core::models::ForkWithTitle>(
        "SELECT f.fork_uri, f.forked_uri, f.vote_score, a.title, a.did
         FROM forks f
         JOIN articles a ON a.at_uri = f.forked_uri
         WHERE f.source_uri = ?
         ORDER BY f.vote_score DESC",
    )
    .bind(&uri)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Get tags
    let tags: Vec<String> = sqlx::query_scalar(
        "SELECT tag_id FROM article_tags WHERE article_uri = ?",
    )
    .bind(&uri)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Ok(Html(templates::article_page(
        &state.config.instance_name,
        &article,
        &html,
        &prereqs,
        &forks,
        &tags,
    )))
}

async fn new_article_page(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let tags = sqlx::query_as::<_, fx_core::models::Tag>("SELECT * FROM tags ORDER BY name")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(templates::new_article_page(
        &state.config.instance_name,
        &tags,
    )))
}

async fn skills_page(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let did = "did:plc:anonymous"; // TODO: from auth
    let skills: Vec<String> = sqlx::query_scalar(
        "SELECT tag_id FROM user_skills WHERE did = ?",
    )
    .bind(did)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let tags = sqlx::query_as::<_, fx_core::models::Tag>("SELECT * FROM tags ORDER BY name")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(templates::skills_page(
        &state.config.instance_name,
        &tags,
        &skills,
    )))
}

async fn graph_page(State(state): State<AppState>) -> Html<String> {
    Html(templates::graph_page(&state.config.instance_name))
}
