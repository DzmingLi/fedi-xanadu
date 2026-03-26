use crate::validation::ValidationError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not found: {entity}: {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("conflict: {message}")]
    Conflict { message: String },

    #[error("validation failed")]
    Validation(Vec<ValidationError>),

    #[error("forbidden: {action}")]
    Forbidden { action: &'static str },

    #[error("unauthorized")]
    Unauthorized,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("pijul error: {0}")]
    Pijul(String),

    #[error("atproto error: {0}")]
    AtProto(String),

    #[error("render error: {0}")]
    Render(String),

    #[error("{0}")]
    Internal(String),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::BadRequest(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_display() {
        let e = Error::NotFound { entity: "article", id: "abc".into() };
        assert_eq!(e.to_string(), "not found: article: abc");
    }

    #[test]
    fn forbidden_display() {
        let e = Error::Forbidden { action: "delete" };
        assert_eq!(e.to_string(), "forbidden: delete");
    }

    #[test]
    fn validation_display() {
        let e = Error::Validation(vec![
            ValidationError { field: "title".into(), message: "empty".into() },
        ]);
        assert_eq!(e.to_string(), "validation failed");
    }

    #[test]
    fn from_serde_json_error() {
        let bad: Result<serde_json::Value, _> = serde_json::from_str("{invalid");
        let err: Error = bad.unwrap_err().into();
        match err {
            Error::BadRequest(msg) => assert!(!msg.is_empty()),
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: Error = io_err.into();
        match err {
            Error::Io(_) => {}
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn validation_error_display() {
        let ve = ValidationError { field: "title".into(), message: "too long".into() };
        assert_eq!(ve.to_string(), "title: too long");
    }
}
