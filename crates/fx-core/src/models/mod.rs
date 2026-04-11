mod article;
mod draft;
mod social;
mod tag;

// Re-export everything flat so existing `use crate::models::*` still works.
pub use article::*;
pub use draft::*;
pub use social::*;
pub use tag::*;

// Re-export enums from content module for convenience.
pub use crate::content::{ContentFormat, ContentKind, PrereqType};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_article_serde() {
        let json = r#"{
            "title": "Test",
            "content": "body",
            "content_format": "typst",
            "tags": ["rust", "math"],
            "prereqs": [{"tag_id": "linear-algebra", "prereq_type": "required"}]
        }"#;
        let input: CreateArticle = serde_json::from_str(json).unwrap();
        assert_eq!(input.title, "Test");
        assert_eq!(input.content_format, ContentFormat::Typst);
        assert_eq!(input.tags.len(), 2);
        assert_eq!(input.prereqs[0].prereq_type, PrereqType::Required);
    }

    #[test]
    fn save_draft_serde() {
        let json = r#"{
            "title": "Draft",
            "content": "wip",
            "content_format": "markdown",
            "tags": [],
            "prereqs": []
        }"#;
        let draft: SaveDraft = serde_json::from_str(json).unwrap();
        assert_eq!(draft.title, "Draft");
        assert_eq!(draft.content_format, ContentFormat::Markdown);
        assert!(draft.lang.is_none());
    }

    #[test]
    fn update_draft_all_optional() {
        let json = r#"{"id": "abc123"}"#;
        let update: UpdateDraft = serde_json::from_str(json).unwrap();
        assert_eq!(update.id, "abc123");
        assert!(update.title.is_none());
        assert!(update.content.is_none());
        assert!(update.tags.is_none());
    }
}
