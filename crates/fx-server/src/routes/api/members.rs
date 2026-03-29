use axum::{extract::State, http::StatusCode, Json};
use fx_core::services::member_service;

use crate::auth::Auth;
use crate::error::ApiResult;
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct MemberInput {
    pub member_did: String,
}

pub async fn add_member(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<MemberInput>,
) -> ApiResult<StatusCode> {
    member_service::add_member(&state.pool, &user.did, &input.member_did).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_member(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<MemberInput>,
) -> ApiResult<StatusCode> {
    member_service::remove_member(&state.pool, &user.did, &input.member_did).await?;
    Ok(StatusCode::OK)
}

pub async fn list_members(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<member_service::Member>>> {
    let members = member_service::list_members(&state.pool, &user.did).await?;
    Ok(Json(members))
}

#[derive(serde::Deserialize)]
pub struct CheckMemberQuery {
    pub author_did: String,
}

pub async fn check_membership(
    State(state): State<AppState>,
    Auth(user): Auth,
    axum::extract::Query(q): axum::extract::Query<CheckMemberQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let is = member_service::is_member(&state.pool, &q.author_did, &user.did).await?;
    Ok(Json(serde_json::json!({ "is_member": is })))
}
