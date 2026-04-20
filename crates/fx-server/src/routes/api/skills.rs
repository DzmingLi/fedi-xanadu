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
use crate::auth::{Auth, WriteAuth, MaybeAuth, pds_put_record, pds_delete_record};
use fx_core::util::now_rfc3339;
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

    let record = serde_json::json!({
        "$type": fx_atproto::lexicon::SKILL,
        "tagId": input.tag_id,
        "status": status,
        "createdAt": now_rfc3339(),
        "updatedAt": now_rfc3339(),
    });
    pds_put_record(&state, &user.token, fx_atproto::lexicon::SKILL, input.tag_id.clone(), record, "light skill").await;

    Ok(StatusCode::NO_CONTENT)
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
    pds_delete_record(&state, &user.token, fx_atproto::lexicon::SKILL, input.tag_id.clone(), "unlight skill").await;
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

#[derive(serde::Deserialize)]
pub struct PrereqInput { from_tag: String, to_tag: String, prereq_type: Option<String> }

pub async fn add_user_tag_prereq(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<PrereqInput>,
) -> ApiResult<StatusCode> {
    let pt = input.prereq_type.as_deref().unwrap_or("required");
    sqlx::query("INSERT INTO user_tag_prereqs (did, from_tag, to_tag, prereq_type) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING")
        .bind(&user.did).bind(&input.from_tag).bind(&input.to_tag).bind(pt)
        .execute(&state.pool).await?;
    Ok(StatusCode::CREATED)
}

#[derive(serde::Deserialize)]
pub struct PrereqDeleteInput { from_tag: String, to_tag: String }

pub async fn remove_user_tag_prereq(
    State(state): State<AppState>,
    Auth(user): Auth,
    Json(input): Json<PrereqDeleteInput>,
) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM user_tag_prereqs WHERE did = $1 AND from_tag = $2 AND to_tag = $3")
        .bind(&user.did).bind(&input.from_tag).bind(&input.to_tag)
        .execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
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
