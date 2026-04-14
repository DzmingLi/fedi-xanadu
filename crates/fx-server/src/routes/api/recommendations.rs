use axum::{
    Json,
    extract::{Query, State},
};
use fx_core::services::recommendation_service;

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::{Auth, MaybeAuth};

#[derive(serde::Deserialize)]
pub struct RecommendationParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub category: Option<String>,
}

pub async fn get_recommendations(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
    Query(params): Query<RecommendationParams>,
) -> ApiResult<Json<Vec<fx_core::models::Article>>> {
    let limit = params.limit.unwrap_or(30).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);
    let category = params.category.as_deref();

    let articles = recommendation_service::get_recommendations(
        &state.pool,
        state.instance_mode,
        user.as_ref().map(|u| u.did.as_str()),
        limit,
        offset,
        category,
    )
    .await?;

    Ok(Json(articles))
}

pub async fn get_recommended_questions(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
    Query(params): Query<RecommendationParams>,
) -> ApiResult<Json<Vec<fx_core::models::Article>>> {
    let limit = params.limit.unwrap_or(8).clamp(1, 20);
    let articles = recommendation_service::get_recommended_questions(
        &state.pool,
        state.instance_mode,
        user.as_ref().map(|u| u.did.as_str()),
        limit,
    )
    .await?;
    Ok(Json(articles))
}

pub async fn get_frontier_skills(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<recommendation_service::FrontierSkill>>> {
    let skills = recommendation_service::get_frontier_skills(&state.pool, &user.did).await?;
    Ok(Json(skills))
}
