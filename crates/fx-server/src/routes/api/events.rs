use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fx_core::services::event_service::{self, CreateEvent, Event, Rsvp, RsvpInput};

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::{Auth, WriteAuth};
use fx_core::util::tid;

#[derive(serde::Deserialize)]
pub struct ListEventsQuery {
    kind: Option<String>,
    tag: Option<String>,
    upcoming: Option<bool>,
    limit: Option<i64>,
    offset: Option<i64>,
}

// ---------------------------------------------------------------------------
// Public
// ---------------------------------------------------------------------------

pub async fn list_events(
    State(state): State<AppState>,
    Query(q): Query<ListEventsQuery>,
) -> ApiResult<Json<Vec<Event>>> {
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    let offset = q.offset.unwrap_or(0).max(0);
    let events = event_service::list_events(
        &state.pool,
        q.kind.as_deref(),
        q.tag.as_deref(),
        q.upcoming,
        limit,
        offset,
    ).await?;
    Ok(Json(events))
}

pub async fn get_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Event>> {
    let event = event_service::get_event(&state.pool, &id).await?;
    Ok(Json(event))
}

pub async fn create_event(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateEvent>,
) -> ApiResult<(StatusCode, Json<Event>)> {
    let id = format!("evt-{}", tid());
    let event = event_service::create_event(&state.pool, &id, &user.did, &input).await?;
    Ok((StatusCode::CREATED, Json(event)))
}

pub async fn update_event(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<CreateEvent>,
) -> ApiResult<Json<Event>> {
    let event = event_service::update_event(&state.pool, &id, &user.did, &input).await?;
    Ok(Json(event))
}

pub async fn cancel_event(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    event_service::cancel_event(&state.pool, &id, &user.did).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn uncancel_event(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    event_service::uncancel_event(&state.pool, &id, &user.did).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_event(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    event_service::delete_event(&state.pool, &id, &user.did).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn rsvp_event(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<RsvpInput>,
) -> ApiResult<StatusCode> {
    let status = input.status.as_deref().unwrap_or("going");
    event_service::rsvp_event(&state.pool, &id, &user.did, status).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn cancel_rsvp(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    event_service::cancel_rsvp(&state.pool, &id, &user.did).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_rsvps(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<Rsvp>>> {
    let rsvps = event_service::list_rsvps(&state.pool, &id).await?;
    Ok(Json(rsvps))
}

pub async fn my_events(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<Event>>> {
    let events = event_service::list_my_events(&state.pool, &user.did).await?;
    Ok(Json(events))
}

pub async fn my_rsvps(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<Vec<Event>>> {
    let events = event_service::my_rsvps(&state.pool, &user.did).await?;
    Ok(Json(events))
}
