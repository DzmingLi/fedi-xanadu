use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use fx_core::services::course_service::{self, CourseRow, CourseListRow, CourseSessionRow, CourseDetailResponse, CourseRatingStats, CourseReviewRow, CreateCourse, UpdateCourse, CreateSession, UpdateSession};
use fx_core::services::patch_service;
use fx_core::util::{tid, now_rfc3339};
use serde::Deserialize;

use crate::auth::{WriteAuth, MaybeAuth, pds_put_record, pds_delete_record};
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
    MaybeAuth(user): MaybeAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<CourseDetailResponse>> {
    let viewer = user.as_ref().map(|u| u.did.as_str());
    let detail = course_service::get_course_detail(&state.pool, &id, viewer).await?;
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
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_series(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    let series_id = q.get("series_id").ok_or(AppError(fx_core::Error::BadRequest("missing series_id".into())))?;
    course_service::remove_series(&state.pool, &id, series_id).await?;
    Ok(StatusCode::NO_CONTENT)
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
    Ok(StatusCode::NO_CONTENT)
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
    fx_core::services::tag_service::require_tag_id(&input.tag_id)?;
    course_service::add_tag(&state.pool, &id, &input.tag_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_tag(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    let tag_id = q.get("tag_id").ok_or(AppError(fx_core::Error::BadRequest("missing tag_id".into())))?;
    fx_core::services::tag_service::require_tag_id(tag_id)?;
    course_service::remove_tag(&state.pool, &id, tag_id).await?;
    Ok(StatusCode::NO_CONTENT)
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
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_textbook(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path(id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    let book_id = q.get("book_id").ok_or(AppError(fx_core::Error::BadRequest("missing book_id".into())))?;
    course_service::remove_textbook(&state.pool, &id, book_id).await?;
    Ok(StatusCode::NO_CONTENT)
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
    fx_core::services::tag_service::require_tag_id(&input.tag_id)?;
    course_service::add_session_tag(&state.pool, &session_id, &input.tag_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_session_tag(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path((_course_id, session_id)): Path<(String, String)>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    let tag_id = q.get("tag_id").ok_or(AppError(fx_core::Error::BadRequest("missing tag_id".into())))?;
    fx_core::services::tag_service::require_tag_id(tag_id)?;
    course_service::remove_session_tag(&state.pool, &session_id, tag_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_session_prereq(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path((_course_id, session_id)): Path<(String, String)>,
    Json(input): Json<SessionTagInput>,
) -> ApiResult<StatusCode> {
    fx_core::services::tag_service::require_tag_id(&input.tag_id)?;
    course_service::add_session_prereq(&state.pool, &session_id, &input.tag_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_session_prereq(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path((_course_id, session_id)): Path<(String, String)>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    let tag_id = q.get("tag_id").ok_or(AppError(fx_core::Error::BadRequest("missing tag_id".into())))?;
    fx_core::services::tag_service::require_tag_id(tag_id)?;
    course_service::remove_session_prereq(&state.pool, &session_id, tag_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Rating & Reviews ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RateCourseInput {
    rating: i16,
}

pub async fn rate_course(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<RateCourseInput>,
) -> ApiResult<Json<CourseRatingStats>> {
    if input.rating < 1 || input.rating > 10 {
        return Err(AppError(fx_core::Error::BadRequest("rating must be 1-10".into())));
    }
    let stats = course_service::rate_course(&state.pool, &id, &user.did, input.rating).await?;

    let record = serde_json::json!({
        "$type": fx_atproto::lexicon::COURSE_RATING,
        "courseId": id,
        "rating": input.rating,
        "createdAt": now_rfc3339(),
        "updatedAt": now_rfc3339(),
    });
    pds_put_record(&state, &user.token, fx_atproto::lexicon::COURSE_RATING, id.clone(), record, "course rate").await;

    Ok(Json(stats))
}

pub async fn unrate_course(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<CourseRatingStats>> {
    let stats = course_service::unrate_course(&state.pool, &id, &user.did).await?;

    pds_delete_record(&state, &user.token, fx_atproto::lexicon::COURSE_RATING, id.clone(), "course unrate").await;

    Ok(Json(stats))
}

// ── Course-level resources ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AddCourseResourceInput {
    pub kind: String,
    pub label: String,
    pub url: String,
    #[serde(default)]
    pub position: i16,
}

pub async fn list_resources(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<course_service::CourseResource>>> {
    let rows = course_service::list_course_resources(&state.pool, &id).await?;
    Ok(Json(rows))
}

pub async fn add_resource(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<AddCourseResourceInput>,
) -> ApiResult<Json<serde_json::Value>> {
    // Homework belongs on the lecture it was released in, not as a
    // free-floating course-level item — otherwise it ends up duplicated
    // in both the calendar and the supplementary panel (CS6110, CS229
    // both had this problem). Block the kind here so it can't recur.
    if input.kind.eq_ignore_ascii_case("homework") {
        return Err(AppError(fx_core::Error::BadRequest(
            "homework cannot be a course-level resource — attach it to the lecture session it belongs to".into(),
        )));
    }
    let new_id = course_service::add_course_resource(
        &state.pool, &id, &input.kind, &input.label, &input.url, input.position, &user.did,
    ).await?;
    Ok(Json(serde_json::json!({ "id": new_id })))
}

pub async fn delete_resource(
    State(state): State<AppState>,
    WriteAuth(_user): WriteAuth,
    Path((_id, resource_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    course_service::delete_course_resource(&state.pool, &resource_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Learning status & session progress ────────────────────────────────

#[derive(Deserialize)]
pub struct LearningStatusInput { pub status: String }

pub async fn set_learning_status(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<LearningStatusInput>,
) -> ApiResult<Json<course_service::CourseLearningStatus>> {
    if !matches!(input.status.as_str(), "want_to_learn" | "learning" | "finished" | "dropped") {
        return Err(AppError(fx_core::Error::BadRequest("invalid status".into())));
    }
    let row = course_service::set_learning_status(&state.pool, &id, &user.did, &input.status).await?;
    Ok(Json(row))
}

pub async fn remove_learning_status(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    course_service::remove_learning_status(&state.pool, &id, &user.did).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct SessionProgressInput { pub session_id: String, pub completed: bool }

pub async fn set_session_progress(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<SessionProgressInput>,
) -> ApiResult<Json<Option<course_service::CourseLearningStatus>>> {
    let status = course_service::set_session_progress(
        &state.pool, &id, &input.session_id, &user.did, input.completed,
    ).await?;

    // Auto-learn: when a session is completed, light up its teaches tags.
    if input.completed {
        let tag_ids: Vec<String> = sqlx::query_scalar(
            "SELECT tag_id FROM course_session_tags WHERE session_id = $1",
        ).bind(&input.session_id).fetch_all(&state.pool).await.unwrap_or_default();
        for tag_id in &tag_ids {
            let _ = fx_core::services::skill_service::light_skill(&state.pool, &user.did, tag_id, "mastered").await;
        }
    }

    Ok(Json(status))
}

#[derive(Deserialize)]
pub struct PageQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(serde::Serialize)]
pub struct PagedReviews {
    pub items: Vec<CourseReviewRow>,
    pub total: i64,
}

#[derive(serde::Serialize)]
pub struct PagedDiscussions {
    pub items: Vec<fx_core::models::Comment>,
    pub total: i64,
}

pub async fn get_reviews(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<PageQuery>,
) -> ApiResult<Json<PagedReviews>> {
    let limit = q.limit.unwrap_or(30).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    let items = course_service::list_course_articles_by_category(&state.pool, &id, "review", limit, offset).await?;
    let total = course_service::count_course_articles_by_category(&state.pool, &id, "review").await?;
    Ok(Json(PagedReviews { items, total }))
}

pub async fn get_notes(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<PageQuery>,
) -> ApiResult<Json<PagedReviews>> {
    let limit = q.limit.unwrap_or(30).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    let items = course_service::list_course_articles_by_category(&state.pool, &id, "note", limit, offset).await?;
    let total = course_service::count_course_articles_by_category(&state.pool, &id, "note").await?;
    Ok(Json(PagedReviews { items, total }))
}

pub async fn get_discussions(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<PageQuery>,
) -> ApiResult<Json<PagedDiscussions>> {
    let limit = q.limit.unwrap_or(30).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    let course_uri = format!("course:{id}");
    let items = fx_core::services::comment_service::list_top_comments(&state.pool, &course_uri, limit, offset).await?;
    let total = fx_core::services::comment_service::count_top_comments(&state.pool, &course_uri).await?;
    Ok(Json(PagedDiscussions { items, total }))
}
