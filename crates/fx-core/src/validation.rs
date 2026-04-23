use crate::Error;

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

pub fn validate_title(title: &str) -> Result<(), ValidationError> {
    if title.is_empty() {
        return Err(ValidationError {
            field: "title".into(),
            message: "title cannot be empty".into(),
        });
    }
    if title.len() > 500 {
        return Err(ValidationError {
            field: "title".into(),
            message: "title must be at most 500 characters".into(),
        });
    }
    Ok(())
}

pub fn validate_body(body: &str, max_len: usize, field_name: &str) -> Result<(), ValidationError> {
    if body.len() > max_len {
        return Err(ValidationError {
            field: field_name.into(),
            message: format!("{field_name} must be at most {max_len} characters"),
        });
    }
    Ok(())
}

pub fn validate_article_content(content: &str) -> Result<(), ValidationError> {
    validate_body(content, 500_000, "content")
}

pub fn validate_comment_body(body: &str) -> Result<(), ValidationError> {
    if body.is_empty() {
        return Err(ValidationError {
            field: "body".into(),
            message: "comment body cannot be empty".into(),
        });
    }
    validate_body(body, 50_000, "body")
}

pub fn validate_folder_path(path: &str) -> Result<(), ValidationError> {
    if path.len() > 255 {
        return Err(ValidationError {
            field: "folder_path".into(),
            message: "folder path must be at most 255 characters".into(),
        });
    }
    if path.contains('\0') || path.contains('\n') {
        return Err(ValidationError {
            field: "folder_path".into(),
            message: "folder path contains invalid characters".into(),
        });
    }
    Ok(())
}

pub fn validate_tag_id(id: &str) -> Result<(), ValidationError> {
    if id.is_empty() {
        return Err(ValidationError {
            field: "tag_id".into(),
            message: "tag id cannot be empty".into(),
        });
    }
    if id.len() > 100 {
        return Err(ValidationError {
            field: "tag_id".into(),
            message: "tag id must be at most 100 characters".into(),
        });
    }
    Ok(())
}

pub fn validate_content_format(format: &str) -> Result<(), ValidationError> {
    match format.parse::<crate::content::ContentFormat>() {
        Err(_) => {
            return Err(ValidationError {
                field: "content_format".into(),
                message: format!("unsupported content format: {format} (allowed: typst, markdown, html)"),
            });
        }
        _ => {}
    }
    Ok(())
}

pub fn validate_at_uri(uri: &str) -> Result<(), ValidationError> {
    if !uri.starts_with("at://") {
        return Err(ValidationError {
            field: "uri".into(),
            message: "URI must start with at://".into(),
        });
    }
    Ok(())
}

/// Validate a create article input and collect all errors.
pub fn validate_create_article(input: &crate::models::CreateArticle) -> Result<(), Error> {
    let mut errors = Vec::new();
    if let Err(e) = validate_title(&input.title) { errors.push(e); }
    // content_format is already a typed enum — no string validation needed
    if let Err(e) = validate_article_content(&input.content) { errors.push(e); }
    for tag in &input.tags {
        if let Err(e) = validate_tag_id(tag) { errors.push(e); }
    }
    if !errors.is_empty() {
        return Err(Error::Validation(errors));
    }
    Ok(())
}

/// Thoughts: title is optional, content limited to 10KB, no HTML format.
pub fn validate_create_thought(input: &crate::models::CreateArticle) -> Result<(), Error> {
    let mut errors = Vec::new();
    // Title is optional for thoughts, but if present must be <= 500
    if input.title.len() > 500 {
        errors.push(ValidationError { field: "title".into(), message: "title too long (max 500)".into() });
    }
    if input.content.is_empty() {
        errors.push(ValidationError { field: "content".into(), message: "content cannot be empty".into() });
    }
    if input.content.len() > 10_000 {
        errors.push(ValidationError { field: "content".into(), message: "thought content too long (max 10000 bytes)".into() });
    }
    if input.content_format == crate::content::ContentFormat::Html {
        errors.push(ValidationError { field: "content_format".into(), message: "thoughts do not support HTML format".into() });
    }
    for tag in &input.tags {
        if let Err(e) = validate_tag_id(tag) { errors.push(e); }
    }
    if !errors.is_empty() {
        return Err(Error::Validation(errors));
    }
    Ok(())
}

pub fn validate_save_draft(input: &crate::models::SaveDraft) -> Result<(), Error> {
    let mut errors = Vec::new();
    if let Err(e) = validate_title(&input.title) { errors.push(e); }
    // content_format is already a typed enum — no string validation needed
    if let Err(e) = validate_article_content(&input.content) { errors.push(e); }
    for tag in &input.tags {
        if let Err(e) = validate_tag_id(tag) { errors.push(e); }
    }
    if !errors.is_empty() {
        return Err(Error::Validation(errors));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- validate_title ---

    #[test]
    fn title_valid() {
        assert!(validate_title("Hello World").is_ok());
    }

    #[test]
    fn title_empty() {
        let err = validate_title("").unwrap_err();
        assert_eq!(err.field, "title");
    }

    #[test]
    fn title_max_length() {
        let t = "a".repeat(500);
        assert!(validate_title(&t).is_ok());
    }

    #[test]
    fn title_too_long() {
        let t = "a".repeat(501);
        assert!(validate_title(&t).is_err());
    }

    // --- validate_body ---

    #[test]
    fn body_within_limit() {
        assert!(validate_body("short", 100, "test").is_ok());
    }

    #[test]
    fn body_exceeds_limit() {
        let err = validate_body(&"x".repeat(101), 100, "test").unwrap_err();
        assert_eq!(err.field, "test");
    }

    // --- validate_article_content ---

    #[test]
    fn article_content_valid() {
        assert!(validate_article_content("some content").is_ok());
    }

    #[test]
    fn article_content_too_long() {
        let c = "x".repeat(500_001);
        assert!(validate_article_content(&c).is_err());
    }

    // --- validate_comment_body ---

    #[test]
    fn comment_body_valid() {
        assert!(validate_comment_body("nice article").is_ok());
    }

    #[test]
    fn comment_body_empty() {
        let err = validate_comment_body("").unwrap_err();
        assert_eq!(err.field, "body");
    }

    #[test]
    fn comment_body_too_long() {
        let b = "x".repeat(50_001);
        assert!(validate_comment_body(&b).is_err());
    }

    // --- validate_folder_path ---

    #[test]
    fn folder_path_valid() {
        assert!(validate_folder_path("我的文章").is_ok());
    }

    #[test]
    fn folder_path_empty_is_ok() {
        assert!(validate_folder_path("").is_ok());
    }

    #[test]
    fn folder_path_too_long() {
        let p = "a".repeat(256);
        assert!(validate_folder_path(&p).is_err());
    }

    #[test]
    fn folder_path_null_byte() {
        assert!(validate_folder_path("foo\0bar").is_err());
    }

    #[test]
    fn folder_path_newline() {
        assert!(validate_folder_path("foo\nbar").is_err());
    }

    // --- validate_tag_id ---

    #[test]
    fn tag_id_valid() {
        assert!(validate_tag_id("rust").is_ok());
    }

    #[test]
    fn tag_id_empty() {
        assert!(validate_tag_id("").is_err());
    }

    #[test]
    fn tag_id_too_long() {
        let id = "x".repeat(101);
        assert!(validate_tag_id(&id).is_err());
    }

    // --- validate_at_uri ---

    #[test]
    fn at_uri_valid() {
        assert!(validate_at_uri("at://did:plc:abc/app.bsky.feed.post/123").is_ok());
    }

    #[test]
    fn at_uri_invalid_prefix() {
        assert!(validate_at_uri("https://example.com").is_err());
    }

    #[test]
    fn at_uri_empty() {
        assert!(validate_at_uri("").is_err());
    }

    // --- validate_create_article ---

    #[test]
    fn create_article_valid() {
        use crate::content::ContentFormat;
        let input = crate::models::CreateArticle {
            title: "Test".into(),
            summary: None,
            content: "Hello".into(),
            content_format: ContentFormat::Typst,
            lang: None,
            license: None,
            translation_of: None,
            restricted: None,
            category: None,
            metadata: None,
            authors: vec![],
            invites: vec![],
            tags: vec!["rust".into()],
            prereqs: vec![],
            related: vec![],
            topics: vec![],
            series_id: None,
            book_chapter_id: None,
            course_session_id: None,
        };
        assert!(validate_create_article(&input).is_ok());
    }

    #[test]
    fn create_article_collects_multiple_errors() {
        use crate::content::ContentFormat;
        let input = crate::models::CreateArticle {
            title: "".into(),
            summary: None,
            content: "x".repeat(500_001),
            content_format: ContentFormat::Typst,
            lang: None,
            license: None,
            translation_of: None,
            restricted: None,
            category: None,
            metadata: None,
            authors: vec![],
            invites: vec![],
            tags: vec!["".into()],
            prereqs: vec![],
            related: vec![],
            topics: vec![],
            series_id: None,
            book_chapter_id: None,
            course_session_id: None,
        };
        match validate_create_article(&input) {
            Err(crate::Error::Validation(errors)) => {
                assert!(errors.len() >= 2, "expected at least 2 errors, got {}", errors.len());
            }
            other => panic!("expected Validation error, got {other:?}"),
        }
    }

    // --- validate_save_draft ---

    #[test]
    fn save_draft_valid() {
        use crate::content::ContentFormat;
        let input = crate::models::SaveDraft {
            title: "Draft".into(),
            summary: None,
            content: "content".into(),
            content_format: ContentFormat::Typst,
            lang: None,
            license: None,
            tags: vec![],
            prereqs: vec![],
        };
        assert!(validate_save_draft(&input).is_ok());
    }

    #[test]
    fn save_draft_empty_title() {
        use crate::content::ContentFormat;
        let input = crate::models::SaveDraft {
            title: "".into(),
            summary: None,
            content: "ok".into(),
            content_format: ContentFormat::Typst,
            lang: None,
            license: None,
            tags: vec![],
            prereqs: vec![],
        };
        assert!(validate_save_draft(&input).is_err());
    }
}
