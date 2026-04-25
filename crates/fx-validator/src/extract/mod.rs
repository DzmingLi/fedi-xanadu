//! Extract `FileMeta` from article source files.
//!
//! Format is selected by path extension:
//! - `*.md`          → [`md`] (YAML frontmatter between `---` fences)
//! - `*/main.typ`    → [`typst`] (`#metadata((...)) <nightboat-translation>`)
//!
//! Callers (knot, indexer) own file walking; this module only parses.

pub mod md;
pub mod typst;
pub mod html;
pub mod series;

use crate::{FileMeta, ValidationError};

/// Label used to identify nightboat article metadata inside a Typst file.
pub const TYPST_LABEL: &str = "nightboat-translation";

/// Label fence used in Markdown YAML frontmatter.
pub const MD_FENCE: &str = "---";

/// Dispatch on file extension. `path` must be a normalized repo-relative path.
pub fn extract(path: &str, content: &str) -> Result<FileMeta, ValidationError> {
    if path.ends_with(".md") {
        md::extract(path, content)
    } else if path == "main.typ" || path.ends_with("/main.typ") {
        typst::extract(path, content)
    } else {
        Err(ValidationError::MetadataParseError {
            path: path.to_string(),
            reason: format!("unsupported file type (expected .md or main.typ)"),
        })
    }
}

/// Returns true if this path is the kind of file the validator should inspect.
pub fn is_article_file(path: &str) -> bool {
    path.ends_with(".md") || path == "main.typ" || path.ends_with("/main.typ")
}
