use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};

use crate::error::ApiResult;
use crate::state::AppState;
use super::{DidQuery, RequireAuth};

#[derive(serde::Serialize)]
pub(crate) struct ProfileResponse {
    did: String,
    handle: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
    article_count: i64,
    series_count: i64,
    links: Vec<ProfileLink>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct ProfileLink {
    label: String,
    url: String,
}

pub async fn get_profile(
    State(state): State<AppState>,
    Query(DidQuery { did }): Query<DidQuery>,
) -> ApiResult<Json<ProfileResponse>> {
    #[derive(sqlx::FromRow)]
    struct ProfileInfo {
        handle: String,
        display_name: Option<String>,
        avatar_url: Option<String>,
        links: String,
    }
    let profile_info = sqlx::query_as::<_, ProfileInfo>(
        "SELECT handle, display_name, avatar_url, links FROM profiles WHERE did = ?"
    )
    .bind(&did)
    .fetch_optional(&state.pool)
    .await?;

    let article_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM articles WHERE did = ?"
    )
    .bind(&did)
    .fetch_one(&state.pool)
    .await?;

    let series_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM series WHERE created_by = ?"
    )
    .bind(&did)
    .fetch_one(&state.pool)
    .await?;

    let links: Vec<ProfileLink> = profile_info
        .as_ref()
        .and_then(|p| serde_json::from_str(&p.links).ok())
        .unwrap_or_default();

    Ok(Json(ProfileResponse {
        did: did.clone(),
        handle: profile_info.as_ref().map(|s| s.handle.clone()),
        display_name: profile_info.as_ref().and_then(|s| s.display_name.clone()),
        avatar_url: profile_info.as_ref().and_then(|s| s.avatar_url.clone()),
        article_count,
        series_count,
        links,
    }))
}

#[derive(serde::Deserialize)]
pub(crate) struct UpdateProfileLinksInput {
    links: Vec<ProfileLink>,
}

pub async fn update_profile_links(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    Json(input): Json<UpdateProfileLinksInput>,
) -> ApiResult<StatusCode> {
    let links_json = serde_json::to_string(&input.links)?;

    sqlx::query("UPDATE profiles SET links = ? WHERE did = ?")
        .bind(&links_json)
        .bind(&did)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::OK)
}
