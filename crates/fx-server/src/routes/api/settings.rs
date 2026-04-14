use axum::{
    Json,
    extract::State,
};

use crate::error::ApiResult;
use crate::state::AppState;
use crate::auth::{Auth, WriteAuth};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct UserSettings {
    pub native_lang: String,
    pub known_langs: Vec<String>,
    pub prefer_native: bool,
    pub hide_unknown: bool,
    pub default_format: String,
    pub email: Option<String>,
    #[serde(default)]
    pub bookmarks_public: bool,
    #[serde(default)]
    pub public_folders: Vec<String>,
    #[serde(default)]
    pub knot_url: Option<String>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            native_lang: "zh".into(),
            known_langs: vec!["zh".into()],
            prefer_native: true,
            hide_unknown: false,
            default_format: "typst".into(),
            email: None,
            bookmarks_public: false,
            public_folders: Vec::new(),
            knot_url: None,
        }
    }
}

pub async fn get_settings(
    State(state): State<AppState>,
    Auth(user): Auth,
) -> ApiResult<Json<UserSettings>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        native_lang: String,
        known_langs: sqlx::types::JsonValue,
        prefer_native: bool,
        hide_unknown: bool,
        default_format: String,
        email: Option<String>,
        bookmarks_public: bool,
        public_folders: sqlx::types::JsonValue,
        knot_url: Option<String>,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT native_lang, known_langs, prefer_native, hide_unknown, default_format, email, \
         bookmarks_public, public_folders, knot_url \
         FROM user_settings WHERE did = $1",
    )
    .bind(&user.did)
    .fetch_optional(&state.pool)
    .await?;

    let settings = match row {
        Some(r) => {
            let known: Vec<String> = serde_json::from_value(r.known_langs).unwrap_or_default();
            let folders: Vec<String> = serde_json::from_value(r.public_folders).unwrap_or_default();
            UserSettings {
                native_lang: r.native_lang,
                known_langs: known,
                prefer_native: r.prefer_native,
                hide_unknown: r.hide_unknown,
                default_format: r.default_format,
                email: r.email,
                bookmarks_public: r.bookmarks_public,
                public_folders: folders,
                knot_url: r.knot_url,
            }
        }
        None => UserSettings::default(),
    };

    Ok(Json(settings))
}

pub async fn set_settings(
    State(state): State<AppState>,
    WriteAuth(user): WriteAuth,
    Json(mut input): Json<UserSettings>,
) -> ApiResult<Json<UserSettings>> {
    // Ensure native_lang is in known_langs
    if !input.known_langs.contains(&input.native_lang) {
        input.known_langs.insert(0, input.native_lang.clone());
    }

    let known_json = serde_json::to_value(&input.known_langs)?;
    let folders_json = serde_json::to_value(&input.public_folders)?;

    sqlx::query(
        "INSERT INTO user_settings (did, native_lang, known_langs, prefer_native, hide_unknown, default_format, email, bookmarks_public, public_folders, knot_url, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW()) \
         ON CONFLICT(did) DO UPDATE SET \
           native_lang = EXCLUDED.native_lang, \
           known_langs = EXCLUDED.known_langs, \
           prefer_native = EXCLUDED.prefer_native, \
           hide_unknown = EXCLUDED.hide_unknown, \
           default_format = EXCLUDED.default_format, \
           email = EXCLUDED.email, \
           bookmarks_public = EXCLUDED.bookmarks_public, \
           public_folders = EXCLUDED.public_folders, \
           knot_url = EXCLUDED.knot_url, \
           updated_at = NOW()",
    )
    .bind(&user.did)
    .bind(&input.native_lang)
    .bind(&known_json)
    .bind(input.prefer_native)
    .bind(input.hide_unknown)
    .bind(&input.default_format)
    .bind(&input.email)
    .bind(input.bookmarks_public)
    .bind(&folders_json)
    .bind(&input.knot_url)
    .execute(&state.pool)
    .await?;

    Ok(Json(input))
}
