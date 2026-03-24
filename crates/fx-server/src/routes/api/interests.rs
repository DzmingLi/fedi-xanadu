use axum::{
    Json,
    extract::State,
    http::StatusCode,
};

use crate::error::ApiResult;
use crate::state::AppState;
use super::AuthDid;

pub async fn get_interests(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
) -> ApiResult<Json<Vec<String>>> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT tag_id FROM user_interests WHERE did = ? ORDER BY tag_id",
    )
    .bind(&did)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(ids))
}

#[derive(serde::Deserialize)]
pub(crate) struct SetInterestsInput {
    tag_ids: Vec<String>,
}

pub async fn set_interests(
    State(state): State<AppState>,
    AuthDid(did): AuthDid,
    Json(input): Json<SetInterestsInput>,
) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM user_interests WHERE did = ?")
        .bind(&did)
        .execute(&state.pool)
        .await?;

    for tag_id in &input.tag_ids {
        let _ = sqlx::query("INSERT OR IGNORE INTO user_interests (did, tag_id) VALUES (?, ?)")
            .bind(&did)
            .bind(tag_id)
            .execute(&state.pool)
            .await;
    }

    Ok(StatusCode::OK)
}
