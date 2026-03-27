use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::models::*;
use fx_core::region::default_visibility;
use fx_core::services::{article_service, notification_service};
use fx_core::validation::validate_create_article;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use super::{WriteAuth, UriQuery, content_hash, tid, uri_to_node_id};

// --- List questions ---

pub async fn list_questions(
    State(state): State<AppState>,
    Query(q): Query<super::articles::ListArticlesQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let offset = q.offset.unwrap_or(0).max(0);
    let questions = article_service::list_questions(&state.pool, state.instance_mode, limit, offset).await?;
    Ok(Json(questions))
}

// --- Get question with answers ---

#[derive(serde::Serialize)]
pub struct QuestionDetail {
    pub question: Article,
    pub answers: Vec<Article>,
}

pub async fn get_question(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<QuestionDetail>> {
    let question = article_service::get_article(&state.pool, state.instance_mode, &uri).await?;
    if question.kind != "question" {
        return Err(AppError(fx_core::Error::NotFound { entity: "question", id: uri }));
    }
    let answers = article_service::list_answers(&state.pool, state.instance_mode, &uri, 100, 0).await?;
    Ok(Json(QuestionDetail { question, answers }))
}

// --- Create question ---

pub async fn create_question(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateArticle>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    validate_create_article(&input)?;

    let at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::ARTICLE, tid());

    let node_id = uri_to_node_id(&at_uri);
    state.pijul.init_repo(&node_id)
        .map_err(|e| AppError(fx_core::Error::Pijul(e.to_string())))?;

    let repo_path = state.pijul.repo_path(&node_id);
    let src_ext = match input.content_format.as_str() {
        "markdown" => "md",
        "html" => "html",
        _ => "typ",
    };
    tokio::fs::write(repo_path.join(format!("content.{src_ext}")), &input.content).await?;

    if input.content_format != "html" {
        let rendered = super::articles::render_content(&input.content_format, &input.content, &repo_path)?;
        let _ = tokio::fs::write(repo_path.join("content.html"), &rendered).await;
    }

    if let Err(e) = state.pijul.record(&node_id, "Initial publish") {
        tracing::warn!("pijul record failed for {node_id}: {e}");
    }

    let hash = content_hash(&input.content);

    let article = article_service::create_article(
        &state.pool, &user.did, &at_uri, &input, &hash, None,
        default_visibility(user.phone_verified), "question", None,
    ).await?;

    let _ = article_service::auto_bookmark(&state.pool, &user.did, &at_uri).await;

    Ok((StatusCode::CREATED, Json(article)))
}

// --- Post answer ---

pub async fn post_answer(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateArticle>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    // translation_of is repurposed: must contain the question_uri
    let question_uri = input.translation_of.as_deref()
        .ok_or_else(|| AppError(fx_core::Error::BadRequest("question_uri is required (pass as translation_of)".into())))?;

    // Verify question exists and is actually a question
    let question = article_service::get_article(&state.pool, state.instance_mode, question_uri).await?;
    if question.kind != "question" {
        return Err(AppError(fx_core::Error::BadRequest("target is not a question".into())));
    }

    validate_create_article(&input)?;

    let at_uri = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::ARTICLE, tid());

    let node_id = uri_to_node_id(&at_uri);
    state.pijul.init_repo(&node_id)
        .map_err(|e| AppError(fx_core::Error::Pijul(e.to_string())))?;

    let repo_path = state.pijul.repo_path(&node_id);
    let src_ext = match input.content_format.as_str() {
        "markdown" => "md",
        "html" => "html",
        _ => "typ",
    };
    tokio::fs::write(repo_path.join(format!("content.{src_ext}")), &input.content).await?;

    if input.content_format != "html" {
        let rendered = super::articles::render_content(&input.content_format, &input.content, &repo_path)?;
        let _ = tokio::fs::write(repo_path.join("content.html"), &rendered).await;
    }

    if let Err(e) = state.pijul.record(&node_id, "Initial publish") {
        tracing::warn!("pijul record failed for {node_id}: {e}");
    }

    let hash = content_hash(&input.content);

    let article = article_service::create_article(
        &state.pool, &user.did, &at_uri, &input, &hash, None,
        default_visibility(user.phone_verified), "answer", Some(question_uri),
    ).await?;

    // Notify question author
    if question.did != user.did {
        let notif_id = tid();
        let _ = notification_service::create_notification(
            &state.pool, &notif_id, &question.did, &user.did,
            "new_answer", Some(question_uri), Some(&at_uri),
        ).await;
    }

    Ok((StatusCode::CREATED, Json(article)))
}

// --- Questions by DID ---

#[derive(serde::Deserialize)]
pub struct DidLimitQuery {
    pub did: String,
    pub limit: Option<i64>,
}

pub async fn get_questions_by_did(
    State(state): State<AppState>,
    Query(q): Query<DidLimitQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let questions = article_service::get_questions_by_did(&state.pool, state.instance_mode, &q.did, limit).await?;
    Ok(Json(questions))
}

pub async fn get_answers_by_did(
    State(state): State<AppState>,
    Query(q): Query<DidLimitQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let answers = article_service::get_answers_by_did(&state.pool, state.instance_mode, &q.did, limit).await?;
    Ok(Json(answers))
}

// --- Questions by tag ---

#[derive(serde::Deserialize)]
pub struct TagQuestionsQuery {
    pub tag_id: String,
    pub limit: Option<i64>,
}

pub async fn get_questions_by_tag(
    State(state): State<AppState>,
    Query(q): Query<TagQuestionsQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let questions = article_service::get_questions_by_tag(&state.pool, state.instance_mode, &q.tag_id, limit).await?;
    Ok(Json(questions))
}
