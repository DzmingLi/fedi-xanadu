use axum::{
    Json,
    extract::State,
    http::HeaderMap,
};

use crate::error::ApiResult;
use crate::state::AppState;
use super::{RequireAuth, session_from_headers, chrono_now};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct KeybindingsData {
    pub bindings: serde_json::Value,
}

pub async fn get_keybindings(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
) -> ApiResult<Json<KeybindingsData>> {
    let bindings: Option<String> = sqlx::query_scalar(
        "SELECT bindings FROM user_keybindings WHERE did = ?",
    )
    .bind(&did)
    .fetch_optional(&state.pool)
    .await?;

    let value = match bindings {
        Some(s) => serde_json::from_str(&s).unwrap_or(serde_json::json!({})),
        None => serde_json::json!({}),
    };

    Ok(Json(KeybindingsData { bindings: value }))
}

pub async fn set_keybindings(
    State(state): State<AppState>,
    RequireAuth(did): RequireAuth,
    headers: HeaderMap,
    Json(input): Json<KeybindingsData>,
) -> ApiResult<Json<KeybindingsData>> {
    let json_str = serde_json::to_string(&input.bindings)?;

    sqlx::query(
        "INSERT INTO user_keybindings (did, bindings, updated_at) VALUES (?, ?, datetime('now'))
         ON CONFLICT(did) DO UPDATE SET bindings = ?, updated_at = datetime('now')",
    )
    .bind(&did)
    .bind(&json_str)
    .bind(&json_str)
    .execute(&state.pool)
    .await?;

    // Sync to PDS
    if let Some((_did, pds_url, access_jwt)) = session_from_headers(&state.pool, &headers).await {
        let record = serde_json::json!({
            "$type": fx_atproto::lexicon::KEYBINDINGS,
            "bindings": input.bindings,
            "updatedAt": chrono_now(),
        });
        let _ = state.at_client.create_record(
            &pds_url,
            &access_jwt,
            &fx_atproto::client::CreateRecordInput {
                repo: _did,
                collection: fx_atproto::lexicon::KEYBINDINGS.to_string(),
                record,
                rkey: Some("self".to_string()),
            },
        ).await;
    }

    Ok(Json(KeybindingsData { bindings: input.bindings }))
}
