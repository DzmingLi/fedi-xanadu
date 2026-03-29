use axum::{
    Json,
    extract::State,
};

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::{Auth, WriteAuth};

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

    Ok(Json(KeybindingsData { bindings: input.bindings }))
}
