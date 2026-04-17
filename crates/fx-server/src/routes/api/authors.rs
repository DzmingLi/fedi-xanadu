use axum::{
    Json,
    extract::{Path, Query, State},
};
use fx_core::services::author_service;

use crate::error::ApiResult;
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<i64>,
}

pub async fn search_authors(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<Vec<author_service::Author>>> {
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let authors = author_service::search_authors(&state.pool, &q.q, limit).await?;
    Ok(Json(authors))
}

pub async fn get_author(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<author_service::AuthorDetail>> {
    let author = author_service::get_author(&state.pool, &id).await?;
    let books = author_service::list_books_by_author(&state.pool, &id).await?;

    // Count articles by this author (via article_authors if DID is linked)
    let article_count: i64 = if let Some(ref did) = author.did {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM article_authors WHERE author_did = $1 AND status != 'rejected'",
        )
        .bind(did)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0)
    } else {
        0
    };

    Ok(Json(author_service::AuthorDetail {
        author,
        books,
        article_count,
    }))
}
