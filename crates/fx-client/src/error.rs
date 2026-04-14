//! Client error types.

/// Result type for client operations.
pub type ClientResult<T> = Result<T, ClientError>;

/// Errors that can occur when using the NightBoat client.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// HTTP transport error (connection refused, timeout, etc.).
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    /// The server returned a non-success status code.
    #[error("api error ({status}): {message}")]
    Api {
        status: u16,
        message: String,
        details: Option<serde_json::Value>,
    },

    /// JSON serialization/deserialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl ClientError {
    /// Returns `true` if this is a 404 Not Found.
    pub fn is_not_found(&self) -> bool {
        matches!(self, ClientError::Api { status: 404, .. })
    }

    /// Returns `true` if this is a 401 Unauthorized.
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, ClientError::Api { status: 401, .. })
    }

    /// Returns `true` if this is a 403 Forbidden.
    pub fn is_forbidden(&self) -> bool {
        matches!(self, ClientError::Api { status: 403, .. })
    }

    /// Returns the HTTP status code if this is an API error.
    pub fn status(&self) -> Option<u16> {
        match self {
            ClientError::Api { status, .. } => Some(*status),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api_error(status: u16) -> ClientError {
        ClientError::Api {
            status,
            message: format!("error {status}"),
            details: None,
        }
    }

    fn api_error_with_details(status: u16, details: serde_json::Value) -> ClientError {
        ClientError::Api {
            status,
            message: format!("error {status}"),
            details: Some(details),
        }
    }

    // ---- is_not_found ----

    #[test]
    fn is_not_found_returns_true_for_404() {
        assert!(api_error(404).is_not_found());
    }

    #[test]
    fn is_not_found_returns_false_for_other_status() {
        assert!(!api_error(401).is_not_found());
        assert!(!api_error(403).is_not_found());
        assert!(!api_error(500).is_not_found());
        assert!(!api_error(200).is_not_found());
    }

    // ---- is_unauthorized ----

    #[test]
    fn is_unauthorized_returns_true_for_401() {
        assert!(api_error(401).is_unauthorized());
    }

    #[test]
    fn is_unauthorized_returns_false_for_other_status() {
        assert!(!api_error(403).is_unauthorized());
        assert!(!api_error(404).is_unauthorized());
        assert!(!api_error(500).is_unauthorized());
    }

    // ---- is_forbidden ----

    #[test]
    fn is_forbidden_returns_true_for_403() {
        assert!(api_error(403).is_forbidden());
    }

    #[test]
    fn is_forbidden_returns_false_for_other_status() {
        assert!(!api_error(401).is_forbidden());
        assert!(!api_error(404).is_forbidden());
        assert!(!api_error(500).is_forbidden());
    }

    // ---- status() ----

    #[test]
    fn status_returns_some_for_api_error() {
        assert_eq!(api_error(404).status(), Some(404));
        assert_eq!(api_error(500).status(), Some(500));
        assert_eq!(api_error(200).status(), Some(200));
    }

    #[test]
    fn status_returns_none_for_json_error() {
        let bad_json: Result<serde_json::Value, _> = serde_json::from_str("not json");
        let err = ClientError::Json(bad_json.unwrap_err());
        assert_eq!(err.status(), None);
    }

    // ---- Display / Error ----

    #[test]
    fn api_error_display_includes_status_and_message() {
        let err = api_error(422);
        let display = format!("{err}");
        assert!(display.contains("422"), "should contain status");
        assert!(display.contains("error 422"), "should contain message");
    }

    #[test]
    fn json_error_display() {
        let bad_json: Result<serde_json::Value, _> = serde_json::from_str("{invalid");
        let err = ClientError::Json(bad_json.unwrap_err());
        let display = format!("{err}");
        assert!(display.contains("json error"), "should mention json");
    }

    // ---- details field ----

    #[test]
    fn api_error_with_details_preserved() {
        let details = serde_json::json!({ "error": "not found", "code": "MISSING" });
        let err = api_error_with_details(404, details.clone());
        match &err {
            ClientError::Api {
                details: Some(d), ..
            } => {
                assert_eq!(d["code"], "MISSING");
            }
            _ => panic!("expected Api error with details"),
        }
    }

    #[test]
    fn api_error_none_details() {
        let err = api_error(500);
        match &err {
            ClientError::Api { details, .. } => assert!(details.is_none()),
            _ => panic!("expected Api error"),
        }
    }

    // ---- From impls ----

    #[test]
    fn from_serde_json_error() {
        let bad: Result<serde_json::Value, _> = serde_json::from_str("}}");
        let serde_err = bad.unwrap_err();
        let client_err: ClientError = serde_err.into();
        assert!(matches!(client_err, ClientError::Json(_)));
        assert_eq!(client_err.status(), None);
        assert!(!client_err.is_not_found());
    }

    // ---- edge cases ----

    #[test]
    fn boundary_status_codes() {
        // 0 and u16::MAX are not real HTTP codes but the enum should handle them
        let err_zero = api_error(0);
        assert_eq!(err_zero.status(), Some(0));
        assert!(!err_zero.is_not_found());
        assert!(!err_zero.is_unauthorized());
        assert!(!err_zero.is_forbidden());

        let err_max = api_error(u16::MAX);
        assert_eq!(err_max.status(), Some(u16::MAX));
    }

    #[test]
    fn methods_are_orthogonal() {
        // Only one of the is_* methods should return true for each status
        for status in [401, 403, 404] {
            let err = api_error(status);
            let checks = [err.is_unauthorized(), err.is_forbidden(), err.is_not_found()];
            let true_count = checks.iter().filter(|&&v| v).count();
            assert_eq!(
                true_count, 1,
                "exactly one is_* should be true for status {status}"
            );
        }
    }
}
