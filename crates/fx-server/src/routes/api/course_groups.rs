use axum::{extract::{Path, State}, http::StatusCode, Json};
use fx_core::services::course_group_service::{
    self, CourseGroup, CourseGroupDetail, CreateCourseGroup,
};

use crate::auth::WriteAuth;
use crate::state::AppState;
use crate::error::ApiResult;

pub async fn list_course_groups(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<CourseGroup>>> {
    Ok(Json(course_group_service::list_course_groups(&state.pool).await?))
}

pub async fn get_course_group(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<CourseGroupDetail>> {
    Ok(Json(course_group_service::get_course_group_detail(&state.pool, &id).await?))
}

pub async fn create_course_group(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateCourseGroup>,
) -> ApiResult<(StatusCode, Json<CourseGroup>)> {
    let group = course_group_service::create_course_group(&state.pool, &input, &user.did).await?;
    Ok((StatusCode::CREATED, Json(group)))
}

pub async fn delete_course_group(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    course_group_service::delete_course_group(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct SetCourseGroupInput {
    pub group_id: Option<String>,
}

/// PUT /courses/{id}/group — assign (or clear) the group for a course.
/// `{"group_id": "cg-xxx"}` assigns; `{"group_id": null}` or DELETE on
/// the same path unassigns.
pub async fn set_course_group(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(course_id): Path<String>,
    Json(input): Json<SetCourseGroupInput>,
) -> ApiResult<StatusCode> {
    course_group_service::set_course_group(&state.pool, &course_id, input.group_id.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unset_course_group(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(course_id): Path<String>,
) -> ApiResult<StatusCode> {
    course_group_service::set_course_group(&state.pool, &course_id, None).await?;
    Ok(StatusCode::NO_CONTENT)
}
