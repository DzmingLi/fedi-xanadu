use axum::{extract::{Path, State}, http::StatusCode, Json};
use fx_core::services::course_service::{
    self, Course, CourseDetail, CourseListItem, CreateCourse,
};

use crate::auth::WriteAuth;
use crate::state::AppState;
use crate::error::ApiResult;

pub async fn list_courses(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<CourseListItem>>> {
    Ok(Json(course_service::list_courses_with_meta(&state.pool).await?))
}

pub async fn get_course(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<CourseDetail>> {
    Ok(Json(course_service::get_course_detail(&state.pool, &id).await?))
}

pub async fn create_course(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateCourse>,
) -> ApiResult<(StatusCode, Json<Course>)> {
    let course = course_service::create_course(&state.pool, &input, &user.did).await?;
    Ok((StatusCode::CREATED, Json(course)))
}

pub async fn delete_course(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    course_service::delete_course(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct SetTermCourseInput {
    pub course_id: Option<String>,
}

/// PUT /terms/{id}/course — assign (or clear) the umbrella course for a term.
/// `{"course_id": "crs-xxx"}` assigns; `{"course_id": null}` or DELETE on
/// the same path unassigns.
pub async fn set_term_course(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(term_id): Path<String>,
    Json(input): Json<SetTermCourseInput>,
) -> ApiResult<StatusCode> {
    course_service::set_term_course(&state.pool, &term_id, input.course_id.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unset_term_course(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(term_id): Path<String>,
) -> ApiResult<StatusCode> {
    course_service::set_term_course(&state.pool, &term_id, None).await?;
    Ok(StatusCode::NO_CONTENT)
}
