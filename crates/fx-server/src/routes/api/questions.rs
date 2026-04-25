use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fx_core::content::{ContentFormat, ContentKind};
use fx_core::models::*;
use fx_core::region::default_visibility;
use fx_core::services::{article_service, notification_service, version_service};
use fx_core::validation::validate_create_article;

use crate::error::{AppError, ApiResult};
use crate::state::AppState;
use crate::auth::WriteAuth;
use fx_core::util::{content_hash, tid, uri_to_node_id};
use super::UriQuery;

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
    if question.kind != ContentKind::Question {
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

    // Q&A is server-only: no pijul repo, no PDS record. We still mint an
    // at_uri-shaped identifier because legacy callers (CLI, admin flows,
    // notifications.target_uri) expect one — `create_article` extracts the
    // rkey to form the server-local `repo_uri` (`server://qa/{rkey}`).
    let local_id = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::WORK, tid());
    let hash = content_hash(&input.content);
    let resolved_desc = input.summary.as_deref().unwrap_or("").to_string();
    let desc_html = crate::summary::render_summary_inline(
        input.content_format.as_str(), &resolved_desc, std::path::Path::new("."),
    ).unwrap_or_default();

    let article = article_service::create_article(
        &state.pool, &user.did, &local_id, &input, &hash, None,
        default_visibility(user.phone_verified), ContentKind::Question, None,
        &resolved_desc, &desc_html,
        None,
    ).await?;

    let _ = article_service::auto_bookmark(&state.pool, &user.did, &local_id).await;

    // Send invite_answer notifications to requested handles. `target_uri`
    // here is the question's synthetic article URI; notification JOIN will
    // resolve the title via articles, not article_localizations.at_uri.
    let target = article.synthetic_uri();
    if !input.invites.is_empty() {
        if let Ok(invited_dids) = notification_service::dids_for_handles(&state.pool, &input.invites).await {
            for did in invited_dids {
                let notif_id = tid();
                let _ = notification_service::create_notification(
                    &state.pool, &notif_id, &did, &user.did,
                    "invite_answer", Some(&target), None,
                ).await;
            }
        }
    }

    Ok((StatusCode::CREATED, Json(article)))
}

// --- Post answer ---

pub async fn post_answer(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(input): Json<CreateArticle>,
) -> ApiResult<(StatusCode, Json<Article>)> {
    // translation_of is repurposed as the question_uri. The caller may pass
    // either the question's synthetic URI (`nightboat://article/...`) or a
    // legacy at_uri; create_article's Q&A branch accepts both.
    let question_uri = input.translation_of.as_deref()
        .ok_or_else(|| AppError(fx_core::Error::BadRequest("question_uri is required (pass as translation_of)".into())))?;

    // Accept either the question's at_uri (legacy/federated) or its synthetic
    // `nightboat://article/...` URI (the only usable handle for server-only
    // Q&A whose localization has at_uri = NULL).
    let question = article_service::resolve_article(&state.pool, question_uri).await?;
    if question.kind != ContentKind::Question {
        return Err(AppError(fx_core::Error::BadRequest("target is not a question".into())));
    }

    validate_create_article(&input)?;

    let local_id = format!("at://{}/{}/{}", user.did, fx_atproto::lexicon::WORK, tid());
    let hash = content_hash(&input.content);
    let resolved_desc = input.summary.as_deref().unwrap_or("").to_string();
    let desc_html = crate::summary::render_summary_inline(
        input.content_format.as_str(), &resolved_desc, std::path::Path::new("."),
    ).unwrap_or_default();

    // NOTE: the `input.translation_of` will reach create_article and, for
    // Q&A (kind=Answer), is routed to the Q&A branch — NOT to the translation
    // branch — because Q&A takes precedence on `matches!(kind, ...)`. The
    // Q&A branch uses `question_uri` (the last positional arg here) to set
    // question_repo_uri / question_source_path correctly.
    let mut input_for_qa = input.clone();
    input_for_qa.translation_of = None; // don't confuse the add_translation_localization branch

    let article = article_service::create_article(
        &state.pool, &user.did, &local_id, &input_for_qa, &hash, None,
        default_visibility(user.phone_verified), ContentKind::Answer, Some(question_uri),
        &resolved_desc, &desc_html,
        None,
    ).await?;

    // Notify question author (skip self-answer).
    if question.author_did != user.did {
        let notif_id = tid();
        let _ = notification_service::create_notification(
            &state.pool, &notif_id, &question.author_did, &user.did,
            "new_answer", Some(question_uri), Some(&article.synthetic_uri()),
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
    let Some(tag_id) = fx_core::services::tag_service::lookup_tag_id(&state.pool, &q.tag_id).await? else {
        return Ok(Json(vec![]));
    };
    let questions = article_service::get_questions_by_tag(&state.pool, state.instance_mode, &tag_id, limit).await?;
    Ok(Json(questions))
}

#[derive(serde::Deserialize)]
pub struct BookQuestionsQuery {
    pub book_id: String,
    pub limit: Option<i64>,
}

pub async fn get_questions_by_book(
    State(state): State<AppState>,
    Query(q): Query<BookQuestionsQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let questions = article_service::get_questions_by_book(&state.pool, state.instance_mode, &q.book_id, limit).await?;
    Ok(Json(questions))
}

pub async fn get_related_questions(
    State(state): State<AppState>,
    Query(UriQuery { uri }): Query<UriQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let related = article_service::get_related_questions(&state.pool, state.instance_mode, &uri, 10).await?;
    Ok(Json(related))
}

#[derive(serde::Deserialize)]
pub struct SessionLimitQuery {
    pub session_id: String,
    pub limit: Option<i64>,
}

pub async fn get_questions_by_session(
    State(state): State<AppState>,
    Query(q): Query<SessionLimitQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let rows = article_service::get_questions_by_session(&state.pool, state.instance_mode, &q.session_id, limit).await?;
    Ok(Json(rows))
}

#[derive(serde::Deserialize)]
pub struct HomeworkLimitQuery {
    pub homework_id: String,
    pub limit: Option<i64>,
}

pub async fn get_questions_by_homework(
    State(state): State<AppState>,
    Query(q): Query<HomeworkLimitQuery>,
) -> ApiResult<Json<Vec<Article>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let rows = article_service::get_questions_by_homework(&state.pool, state.instance_mode, &q.homework_id, limit).await?;
    Ok(Json(rows))
}
