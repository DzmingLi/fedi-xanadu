use axum::{
    Json,
    extract::State,
    http::StatusCode,
};

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use super::{Auth, WriteAuth};

const MAX_INTERESTS: usize = 100;

pub async fn get_interests(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<String>>> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT tag_id FROM user_interests WHERE did = $1 ORDER BY tag_id",
    )
    .bind(&user.did)
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
    WriteAuth(user): WriteAuth,
    Json(input): Json<SetInterestsInput>,
) -> ApiResult<StatusCode> {
    if input.tag_ids.len() > MAX_INTERESTS {
        return Err(AppError(fx_core::Error::BadRequest(
            format!("too many interests (max {MAX_INTERESTS})")
        )));
    }

    let mut tx = state.pool.begin().await?;

    sqlx::query("DELETE FROM user_interests WHERE did = $1")
        .bind(&user.did)
        .execute(&mut *tx)
        .await?;

    for tag_id in &input.tag_ids {
        sqlx::query("INSERT INTO user_interests (did, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(&user.did)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(StatusCode::OK)
}
