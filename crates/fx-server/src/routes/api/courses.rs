use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use fx_core::services::course_service::{self, CourseRow, CourseListRow, CourseSessionRow, CourseDetailResponse, CreateCourse, UpdateCourse, CreateSession, UpdateSession};
use fx_core::services::patch_service;
use fx_core::util::tid;
use serde::Deserialize;

use crate::auth::WriteAuth;
use crate::state::AppState;
use crate::error::{ApiResult, AppError};

pub async fn list_courses(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<CourseListRow>>> {
    let courses = course_service::list_courses(&state.pool).await?;
    Ok(Json(courses))
}

pub async fn my_courses(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
) -> ApiResult<Json<Vec<CourseListRow>>> {
    let courses = course_service::list_my_courses(&state.pool, &user.did).await?;
    Ok(Json(courses))
}

pub async fn get_course(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<CourseDetailResponse>> {
    let detail = course_service::get_course_detail(&state.pool, &id).await?;
    Ok(Json(detail))
}

pub async fn create_course(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateCourse>,
) -> ApiResult<(StatusCode, Json<CourseRow>)> {
    if let Err(e) = fx_core::validation::validate_title(&input.title) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    let id = format!("crs-{}", tid());
    let course = course_service::create_course(&state.pool, &id, &user.did, &input).await?;

    // Auto-add creator as instructor
    let _ = course_service::add_staff(&state.pool, &id, &user.did, "instructor").await;

    Ok((StatusCode::CREATED, Json(course)))
}

pub async fn update_course(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<UpdateCourse>,
) -> ApiResult<Json<CourseRow>> {
    let summary = input.summary.as_deref().unwrap_or("");
    let course = course_service::update_course(&state.pool, &id, &user.did, &input, summary).await?;
    Ok(Json(course))
}

pub async fn delete_course(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    course_service::delete_course(&state.pool, &id, &user.did).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Relation endpoints ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AddSeriesInput {
    series_id: String,
    role: Option<String>,
    sort_order: Option<i32>,
}

pub async fn add_series(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<AddSeriesInput>,
) -> ApiResult<StatusCode> {
    course_service::add_series(
        &state.pool, &id, &input.series_id,
        input.role.as_deref().unwrap_or("lectures"),
        input.sort_order.unwrap_or(0),
    ).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_series(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    let series_id = q.get("series_id").ok_or(AppError(fx_core::Error::BadRequest("missing series_id".into())))?;
    course_service::remove_series(&state.pool, &id, series_id).await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct AddStaffInput {
    user_did: String,
    role: Option<String>,
}

pub async fn add_staff(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<AddStaffInput>,
) -> ApiResult<StatusCode> {
    course_service::add_staff(&state.pool, &id, &input.user_did, input.role.as_deref().unwrap_or("ta")).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_staff(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    let user_did = q.get("user_did").ok_or(AppError(fx_core::Error::BadRequest("missing user_did".into())))?;
    course_service::remove_staff(&state.pool, &id, user_did).await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct AddSkillTreeInput {
    tree_uri: String,
    role: Option<String>,
}

pub async fn add_skill_tree(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<AddSkillTreeInput>,
) -> ApiResult<StatusCode> {
    course_service::add_skill_tree(&state.pool, &id, &input.tree_uri, input.role.as_deref().unwrap_or("prerequisites")).await?;
    Ok(StatusCode::OK)
}

pub async fn list_patches(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<patch_service::PatchRow>>> {
    let patches = patch_service::list_patches(&state.pool, "course", &id).await?;
    Ok(Json(patches))
}

#[derive(Deserialize)]
pub struct AddTagInput {
    tag_id: String,
}

pub async fn add_tag(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<AddTagInput>,
) -> ApiResult<StatusCode> {
    course_service::add_tag(&state.pool, &id, &input.tag_id).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_tag(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    let tag_id = q.get("tag_id").ok_or(AppError(fx_core::Error::BadRequest("missing tag_id".into())))?;
    course_service::remove_tag(&state.pool, &id, tag_id).await?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct AddTextbookInput {
    book_id: String,
    role: Option<String>,
    sort_order: Option<i32>,
}

pub async fn add_textbook(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<AddTextbookInput>,
) -> ApiResult<StatusCode> {
    course_service::add_textbook(&state.pool, &id, &input.book_id, input.role.as_deref().unwrap_or("required"), input.sort_order.unwrap_or(0)).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_textbook(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    let book_id = q.get("book_id").ok_or(AppError(fx_core::Error::BadRequest("missing book_id".into())))?;
    course_service::remove_textbook(&state.pool, &id, book_id).await?;
    Ok(StatusCode::OK)
}

// ── Session endpoints ──────────────────────────────────────────────────

pub async fn create_session(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(course_id): Path<String>,
    Json(input): Json<CreateSession>,
) -> ApiResult<(StatusCode, Json<CourseSessionRow>)> {
    let session_id = format!("csn-{}", tid());
    let session = course_service::create_session(&state.pool, &session_id, &course_id, &input).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

pub async fn update_session(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path((_course_id, session_id)): Path<(String, String)>,
    Json(input): Json<UpdateSession>,
) -> ApiResult<Json<CourseSessionRow>> {
    let session = course_service::update_session(&state.pool, &session_id, &input).await?;
    Ok(Json(session))
}

pub async fn delete_session(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path((_course_id, session_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    course_service::delete_session(&state.pool, &session_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct SessionTagInput {
    tag_id: String,
}

pub async fn add_session_tag(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path((_course_id, session_id)): Path<(String, String)>,
    Json(input): Json<SessionTagInput>,
) -> ApiResult<StatusCode> {
    course_service::add_session_tag(&state.pool, &session_id, &input.tag_id).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_session_tag(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path((_course_id, session_id)): Path<(String, String)>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    let tag_id = q.get("tag_id").ok_or(AppError(fx_core::Error::BadRequest("missing tag_id".into())))?;
    course_service::remove_session_tag(&state.pool, &session_id, tag_id).await?;
    Ok(StatusCode::OK)
}

pub async fn add_session_prereq(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path((_course_id, session_id)): Path<(String, String)>,
    Json(input): Json<SessionTagInput>,
) -> ApiResult<StatusCode> {
    course_service::add_session_prereq(&state.pool, &session_id, &input.tag_id).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_session_prereq(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path((_course_id, session_id)): Path<(String, String)>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    let tag_id = q.get("tag_id").ok_or(AppError(fx_core::Error::BadRequest("missing tag_id".into())))?;
    course_service::remove_session_prereq(&state.pool, &session_id, tag_id).await?;
    Ok(StatusCode::OK)
}
