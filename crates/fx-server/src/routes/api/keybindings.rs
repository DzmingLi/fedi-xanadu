use axum::{
    Json,
    extract::State,
};

use crate::error::ApiResult;
use crate::state::AppState;
use super::{Auth, WriteAuth, pds_session, now_rfc3339, log_pds_error};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct KeybindingsData {
    pub bindings: serde_json::Value,
}

pub async fn get_keybindings(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<KeybindingsData>> {
    let bindings: Option<String> = sqlx::query_scalar(
        "SELECT bindings FROM user_keybindings WHERE did = $1",
    )
    .bind(&user.did)
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
    WriteAuth(user): WriteAuth,
    Json(input): Json<KeybindingsData>,
) -> ApiResult<Json<KeybindingsData>> {
    let json_str = serde_json::to_string(&input.bindings)?;

    sqlx::query(
        "INSERT INTO user_keybindings (did, bindings, updated_at) VALUES ($1, $2, NOW())
         ON CONFLICT(did) DO UPDATE SET bindings = EXCLUDED.bindings, updated_at = NOW()",
    )
    .bind(&user.did)
    .bind(&json_str)
    .execute(&state.pool)
    .await?;

    // Sync to PDS
    if let Some(pds) = pds_session(&state.pool, &user.token).await {
        let record = serde_json::json!({
            "$type": fx_atproto::lexicon::KEYBINDINGS,
            "bindings": input.bindings,
            "updatedAt": now_rfc3339(),
        });
        if let Err(e) = state.at_client.create_record(
            &pds.pds_url,
            &pds.access_jwt,
            &fx_atproto::client::CreateRecordInput {
                repo: pds.did,
                collection: fx_atproto::lexicon::KEYBINDINGS.to_string(),
                record,
                rkey: Some("self".to_string()),
            },
        ).await {
            log_pds_error("sync keybindings", e);
        }
    }

    Ok(Json(KeybindingsData { bindings: input.bindings }))
}
