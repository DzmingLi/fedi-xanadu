use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Thin newtype over `fx_core::Error` that implements `IntoResponse`.
/// This keeps the domain error in fx-core free of web framework dependencies.
#[derive(Debug)]
pub struct AppError(pub fx_core::Error);

pub type ApiResult<T> = Result<T, AppError>;

impl From<fx_core::Error> for AppError {
    fn from(e: fx_core::Error) -> Self {
        AppError(e)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError(fx_core::Error::Database(e))
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError(fx_core::Error::Io(e))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError(fx_core::Error::BadRequest(e.to_string()))
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError(fx_core::Error::Internal(e.to_string()))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        use fx_core::Error;

        match self.0 {
            Error::Validation(errors) => {
                let details: Vec<_> = errors
                    .iter()
                    .map(|e| serde_json::json!({ "field": e.field, "message": e.message }))
                    .collect();
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({ "error": "validation_failed", "details": details })),
                )
                    .into_response()
            }
            Error::NotFound { entity, id } => {
                let msg = format!("{entity}: {id}");
                (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": msg }))).into_response()
            }
            Error::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": msg }))).into_response()
            }
            Error::Unauthorized => {
                (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "unauthorized" })))
                    .into_response()
            }
            Error::Forbidden { action } => {
                tracing::warn!("forbidden: {action}");
                (StatusCode::FORBIDDEN, Json(serde_json::json!({ "error": "forbidden" })))
                    .into_response()
            }
            Error::Conflict { message } => {
                (StatusCode::CONFLICT, Json(serde_json::json!({ "error": message }))).into_response()
            }
            Error::Database(e) => {
                tracing::error!("database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "database error" })),
                )
                    .into_response()
            }
            Error::Io(e) => {
                tracing::error!("io error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "io error" })),
                )
                    .into_response()
            }
            Error::Pijul(msg) => {
                tracing::error!("pijul error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "version control error" })),
                )
                    .into_response()
            }
            Error::AtProto(msg) => {
                tracing::error!("atproto error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "federation error" })),
                )
                    .into_response()
            }
            Error::Render(msg) => {
                (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": msg }))).into_response()
            }
            Error::Internal(msg) => {
                tracing::error!("internal error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": msg })),
                )
                    .into_response()
            }
        }
    }
}

/// Helper to verify resource ownership.
pub fn require_owner(owner: Option<&str>, did: &str) -> Result<(), AppError> {
    match owner {
        Some(o) if o == did => Ok(()),
        Some(_) => Err(AppError(fx_core::Error::Forbidden {
            action: "access resource owned by another user",
        })),
        None => Err(AppError(fx_core::Error::NotFound {
            entity: "resource",
            id: "unknown".to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    fn status_of(err: fx_core::Error) -> StatusCode {
        AppError(err).into_response().status()
    }

    #[test]
    fn not_found_maps_to_404() {
        assert_eq!(
            status_of(fx_core::Error::NotFound { entity: "article", id: "x".into() }),
            StatusCode::NOT_FOUND,
        );
    }

    #[test]
    fn unauthorized_maps_to_401() {
        assert_eq!(status_of(fx_core::Error::Unauthorized), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn forbidden_maps_to_403() {
        assert_eq!(
            status_of(fx_core::Error::Forbidden { action: "test" }),
            StatusCode::FORBIDDEN,
        );
    }

    #[test]
    fn bad_request_maps_to_400() {
        assert_eq!(
            status_of(fx_core::Error::BadRequest("bad".into())),
            StatusCode::BAD_REQUEST,
        );
    }

    #[test]
    fn validation_maps_to_422() {
        assert_eq!(
            status_of(fx_core::Error::Validation(vec![])),
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }

    #[test]
    fn conflict_maps_to_409() {
        assert_eq!(
            status_of(fx_core::Error::Conflict { message: "dup".into() }),
            StatusCode::CONFLICT,
        );
    }

    #[test]
    fn internal_maps_to_500() {
        assert_eq!(
            status_of(fx_core::Error::Internal("oops".into())),
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    }

    #[test]
    fn render_maps_to_400() {
        assert_eq!(
            status_of(fx_core::Error::Render("bad typst".into())),
            StatusCode::BAD_REQUEST,
        );
    }

    // --- require_owner ---

    #[test]
    fn require_owner_same_did() {
        assert!(require_owner(Some("did:plc:abc"), "did:plc:abc").is_ok());
    }

    #[test]
    fn require_owner_different_did() {
        let err = require_owner(Some("did:plc:abc"), "did:plc:xyz").unwrap_err();
        assert_eq!(err.into_response().status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn require_owner_none() {
        let err = require_owner(None, "did:plc:abc").unwrap_err();
        assert_eq!(err.into_response().status(), StatusCode::NOT_FOUND);
    }
}
