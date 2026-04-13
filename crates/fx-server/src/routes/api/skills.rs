use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use fx_core::models::UserSkill;
use fx_core::services::skill_service;
use fx_core::validation;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::{WriteAuth, MaybeAuth};
use super::TagIdQuery;

pub async fn list_user_skills(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
) -> ApiResult<Json<Vec<UserSkill>>> {
    let did = user.map(|u| u.did).unwrap_or_default();
    let skills = skill_service::list_user_skills(&state.pool, &did).await?;
    Ok(Json(skills))
}

#[derive(serde::Deserialize)]
pub struct LightSkillInput {
    tag_id: String,
    status: Option<String>,
}

pub async fn light_skill(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<LightSkillInput>,
) -> ApiResult<StatusCode> {
    if let Err(e) = validation::validate_tag_id(&input.tag_id) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    let status = input.status.as_deref().unwrap_or("mastered");
    skill_service::light_skill(&state.pool, &user.did, &input.tag_id, status).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_skill(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<TagIdQuery>,
) -> ApiResult<StatusCode> {
    if let Err(e) = validation::validate_tag_id(&input.tag_id) {
        return Err(AppError(fx_core::Error::Validation(vec![e])));
    }

    skill_service::delete_skill(&state.pool, &user.did, &input.tag_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- User Tag Prereqs ---

pub async fn get_user_tag_prereqs(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
) -> ApiResult<Json<Vec<skill_service::UserTagPrereq>>> {
    let did = user.map(|u| u.did).unwrap_or_default();
    let prereqs = skill_service::get_user_tag_prereqs(&state.pool, &did).await?;
    Ok(Json(prereqs))
}

// --- User Tag Tree ---

pub async fn get_user_tag_tree(
    State(state): State<AppState>,
    MaybeAuth(user): MaybeAuth,
) -> ApiResult<Json<Vec<skill_service::TagTreeEntry>>> {
    let did = user.map(|u| u.did).unwrap_or_default();
    let tree = skill_service::get_user_tag_tree(&state.pool, &did).await?;
    Ok(Json(tree))
}

#[derive(serde::Deserialize)]
pub struct AddTagChildInput {
    parent_tag: String,
    child_tag: String,
}

pub async fn add_tag_child(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<AddTagChildInput>,
) -> ApiResult<StatusCode> {
    let mut errors = Vec::new();
    if let Err(e) = validation::validate_tag_id(&input.parent_tag) {
        errors.push(e);
    }
    if let Err(e) = validation::validate_tag_id(&input.child_tag) {
        errors.push(e);
    }
    if !errors.is_empty() {
        return Err(AppError(fx_core::Error::Validation(errors)));
    }

    skill_service::add_tag_child(&state.pool, &user.did, &input.parent_tag, &input.child_tag).await?;
    Ok(StatusCode::CREATED)
}
