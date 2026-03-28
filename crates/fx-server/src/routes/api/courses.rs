use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::services::course_service;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use super::{Auth, WriteAuth, tid};

// --- List ---

#[derive(serde::Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_courses(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<course_service::Course>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    let rows = course_service::list_courses(&state.pool, limit, offset).await?;
    Ok(Json(rows))
}

// --- Get detail ---

#[derive(serde::Deserialize)]
pub struct IdQuery { pub id: String }

pub async fn get_course(
    State(state): State<AppState>,
    Query(q): Query<IdQuery>,
) -> ApiResult<Json<course_service::CourseDetail>> {
    let detail = course_service::get_course(&state.pool, &q.id).await?;
    Ok(Json(detail))
}

// --- Create ---

pub async fn create_course(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<course_service::CreateCourse>,
) -> ApiResult<(StatusCode, Json<course_service::Course>)> {
    if input.title.trim().is_empty() {
        return Err(AppError(fx_core::Error::BadRequest("title required".into())));
    }
    let id = format!("c-{}", tid());
    let course = course_service::create_course(&state.pool, &id, &input, &user.did).await?;
    Ok((StatusCode::CREATED, Json(course)))
}

// --- Update ---

#[derive(serde::Deserialize)]
pub struct UpdateCourseInput {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub schedule_type: Option<String>,
    pub term: Option<Option<String>>,
    pub year: Option<Option<i16>>,
}

pub async fn update_course(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<UpdateCourseInput>,
) -> ApiResult<StatusCode> {
    let owner = course_service::get_course_owner(&state.pool, &input.id).await?;
    if owner != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "not course owner" }));
    }
    course_service::update_course(
        &state.pool, &input.id,
        input.title.as_deref(), input.description.as_deref(),
        input.cover_url.as_deref(), input.schedule_type.as_deref(),
        input.term.as_ref().map(|o| o.as_deref()),
        input.year,
    ).await?;
    Ok(StatusCode::OK)
}

// --- Delete ---

#[derive(serde::Deserialize)]
pub struct DeleteInput { pub id: String }

pub async fn delete_course(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<DeleteInput>,
) -> ApiResult<StatusCode> {
    let owner = course_service::get_course_owner(&state.pool, &input.id).await?;
    if owner != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "not course owner" }));
    }
    course_service::delete_course(&state.pool, &input.id).await?;
    Ok(StatusCode::OK)
}

// --- Units ---

#[derive(serde::Deserialize)]
pub struct CreateUnitInput {
    pub course_id: String,
    pub title: String,
    pub description: Option<String>,
    pub available_from: Option<chrono::NaiveDate>,
}

pub async fn create_unit(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<CreateUnitInput>,
) -> ApiResult<(StatusCode, Json<course_service::CourseUnit>)> {
    let owner = course_service::get_course_owner(&state.pool, &input.course_id).await?;
    if owner != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "not course owner" }));
    }
    let id = format!("cu-{}", tid());
    let unit = course_service::create_unit(
        &state.pool, &id, &input.course_id,
        &input.title, input.description.as_deref().unwrap_or(""),
        input.available_from,
    ).await?;
    Ok((StatusCode::CREATED, Json(unit)))
}

#[derive(serde::Deserialize)]
pub struct UpdateUnitInput {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub available_from: Option<Option<chrono::NaiveDate>>,
}

pub async fn update_unit(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<UpdateUnitInput>,
) -> ApiResult<StatusCode> {
    let course_id = course_service::get_unit_course_id(&state.pool, &input.id).await?;
    let owner = course_service::get_course_owner(&state.pool, &course_id).await?;
    if owner != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "not course owner" }));
    }
    course_service::update_unit(
        &state.pool, &input.id,
        input.title.as_deref(), input.description.as_deref(),
        input.available_from,
    ).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_unit(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<DeleteInput>,
) -> ApiResult<StatusCode> {
    let course_id = course_service::get_unit_course_id(&state.pool, &input.id).await?;
    let owner = course_service::get_course_owner(&state.pool, &course_id).await?;
    if owner != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "not course owner" }));
    }
    course_service::delete_unit(&state.pool, &input.id).await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub struct ReorderUnitsInput {
    pub course_id: String,
    pub unit_ids: Vec<String>,
}

pub async fn reorder_units(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<ReorderUnitsInput>,
) -> ApiResult<StatusCode> {
    let owner = course_service::get_course_owner(&state.pool, &input.course_id).await?;
    if owner != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "not course owner" }));
    }
    course_service::reorder_units(&state.pool, &input.course_id, &input.unit_ids).await?;
    Ok(StatusCode::OK)
}

// --- Items ---

#[derive(serde::Deserialize)]
pub struct CreateItemInput {
    pub unit_id: String,
    pub title: String,
    pub role: Option<String>,
    pub target_uri: Option<String>,
    pub external_url: Option<String>,
    pub note: Option<String>,
    pub due_date: Option<chrono::NaiveDate>,
}

pub async fn create_item(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<CreateItemInput>,
) -> ApiResult<(StatusCode, Json<course_service::CourseItem>)> {
    let course_id = course_service::get_unit_course_id(&state.pool, &input.unit_id).await?;
    let owner = course_service::get_course_owner(&state.pool, &course_id).await?;
    if owner != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "not course owner" }));
    }
    let id = format!("ci-{}", tid());
    let item = course_service::create_item(
        &state.pool, &id, &input.unit_id,
        input.role.as_deref().unwrap_or("reading"),
        input.target_uri.as_deref(), input.external_url.as_deref(),
        &input.title, input.note.as_deref().unwrap_or(""),
        input.due_date,
    ).await?;
    Ok((StatusCode::CREATED, Json(item)))
}

#[derive(serde::Deserialize)]
pub struct UpdateItemInput {
    pub id: String,
    pub title: Option<String>,
    pub role: Option<String>,
    pub target_uri: Option<Option<String>>,
    pub external_url: Option<Option<String>>,
    pub note: Option<String>,
    pub due_date: Option<Option<chrono::NaiveDate>>,
}

pub async fn update_item(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<UpdateItemInput>,
) -> ApiResult<StatusCode> {
    let unit_id = course_service::get_item_unit_id(&state.pool, &input.id).await?;
    let course_id = course_service::get_unit_course_id(&state.pool, &unit_id).await?;
    let owner = course_service::get_course_owner(&state.pool, &course_id).await?;
    if owner != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "not course owner" }));
    }
    course_service::update_item(
        &state.pool, &input.id,
        input.role.as_deref(),
        input.target_uri.as_ref().map(|o| o.as_deref()),
        input.external_url.as_ref().map(|o| o.as_deref()),
        input.title.as_deref(), input.note.as_deref(),
        input.due_date,
    ).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_item(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<DeleteInput>,
) -> ApiResult<StatusCode> {
    let unit_id = course_service::get_item_unit_id(&state.pool, &input.id).await?;
    let course_id = course_service::get_unit_course_id(&state.pool, &unit_id).await?;
    let owner = course_service::get_course_owner(&state.pool, &course_id).await?;
    if owner != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "not course owner" }));
    }
    course_service::delete_item(&state.pool, &input.id).await?;
    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub struct ReorderItemsInput {
    pub unit_id: String,
    pub item_ids: Vec<String>,
}

pub async fn reorder_items(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<ReorderItemsInput>,
) -> ApiResult<StatusCode> {
    let course_id = course_service::get_unit_course_id(&state.pool, &input.unit_id).await?;
    let owner = course_service::get_course_owner(&state.pool, &course_id).await?;
    if owner != user.did {
        return Err(AppError(fx_core::Error::Forbidden { action: "not course owner" }));
    }
    course_service::reorder_items(&state.pool, &input.unit_id, &input.item_ids).await?;
    Ok(StatusCode::OK)
}
