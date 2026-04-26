use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use fx_core::services::term_homework_service::{
    self, TermHomework, CreateHomework, UpdateHomework,
};

use crate::auth::WriteAuth;
use crate::state::AppState;
use crate::error::ApiResult;

pub async fn create_homework(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateHomework>,
) -> ApiResult<(StatusCode, Json<TermHomework>)> {
    let hw = term_homework_service::create_homework(&state.pool, &input, &user.did).await?;
    Ok((StatusCode::CREATED, Json(hw)))
}

#[derive(serde::Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub term_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}

/// GET /homeworks?term_id=...  or  ?session_id=...
/// Exactly one must be supplied. Returns the term's full homework list
/// (ordered by position), or the narrower session-scoped subset.
pub async fn list_homeworks(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<TermHomework>>> {
    let rows = match (q.term_id.as_deref(), q.session_id.as_deref()) {
        (Some(cid), None) => term_homework_service::list_homeworks_by_term(&state.pool, cid).await?,
        (None, Some(sid)) => term_homework_service::list_homeworks_by_session(&state.pool, sid).await?,
        _ => return Err(crate::error::AppError(fx_core::Error::BadRequest(
            "provide exactly one of term_id or session_id".into()
        ))),
    };
    Ok(Json(rows))
}

pub async fn get_homework(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<TermHomework>> {
    Ok(Json(term_homework_service::get_homework(&state.pool, &id).await?))
}

pub async fn update_homework(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<UpdateHomework>,
) -> ApiResult<Json<TermHomework>> {
    Ok(Json(term_homework_service::update_homework(&state.pool, &id, &input).await?))
}

pub async fn delete_homework(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    term_homework_service::delete_homework(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
