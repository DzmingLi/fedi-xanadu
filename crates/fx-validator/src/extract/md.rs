//! YAML frontmatter extraction for `*.md` article files.

use crate::{FileMeta, Format, ValidationError};
use serde::Deserialize;

use super::MD_FENCE;

/// Raw frontmatter shape as it appears in the file.
///
/// Kept separate from [`FileMeta`] so that (a) the on-disk field names can use
/// kebab-case (idiomatic for YAML/Typst) while the Rust API uses snake_case, and
/// (b) the `path`/`format` fields are not author-supplied.
#[derive(Debug, Default, Deserialize)]
struct RawFrontmatter {
    lang: Option<String>,
    #[serde(alias = "translation-of", alias = "translation_of")]
    translation_of: Option<String>,
    title: Option<String>,
    #[serde(rename = "abstract")]
    abstract_: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    translator: Option<String>,
    #[serde(alias = "translation-notes", alias = "translation_notes")]
    translation_notes: Option<String>,
}

pub fn extract(path: &str, content: &str) -> Result<FileMeta, ValidationError> {
    let raw = parse_frontmatter(path, content)?;

    let lang = raw.lang.ok_or_else(|| ValidationError::MissingLang {
        path: path.to_string(),
    })?;

    Ok(FileMeta {
        path: path.to_string(),
        format: Format::Md,
        lang,
        translation_of: raw.translation_of,
        title: raw.title,
        abstract_: raw.abstract_,
        tags: raw.tags,
        translator: raw.translator,
        translation_notes: raw.translation_notes,
    })
}

fn parse_frontmatter(path: &str, content: &str) -> Result<RawFrontmatter, ValidationError> {
    // Require `---\n` as the VERY first line. No leading whitespace / BOM tolerated —
    // keep the format tight so tooling is predictable.
    let Some(after_open) = strip_opening_fence(content) else {
        return Err(ValidationError::MetadataMissing {
            path: path.to_string(),
        });
    };

    let Some(end) = find_closing_fence(after_open) else {
        return Err(ValidationError::MetadataParseError {
            path: path.to_string(),
            reason: "unterminated YAML frontmatter (missing closing `---`)".into(),
        });
    };

    let yaml = &after_open[..end];
    serde_yml::from_str::<RawFrontmatter>(yaml).map_err(|e| ValidationError::MetadataParseError {
        path: path.to_string(),
        reason: format!("YAML parse error: {e}"),
    })
}

fn strip_opening_fence(content: &str) -> Option<&str> {
    let first_line_end = content.find('\n')?;
    let first_line = &content[..first_line_end];
    if first_line.trim_end_matches('\r') == MD_FENCE {
        Some(&content[first_line_end + 1..])
    } else {
        None
    }
}

/// Returns the byte offset (within `body`) of the line that starts with `---`.
fn find_closing_fence(body: &str) -> Option<usize> {
    let mut offset = 0usize;
    for line in body.split_inclusive('\n') {
        let stripped = line.trim_end_matches('\n').trim_end_matches('\r');
        if stripped == MD_FENCE {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_source_metadata() {
        let md = "---\nlang: zh-CN\ntitle: Hello\n---\n\n# Body\n";
        let meta = extract("foo.md", md).unwrap();
        assert_eq!(meta.lang, "zh-CN");
        assert_eq!(meta.title.as_deref(), Some("Hello"));
        assert_eq!(meta.translation_of, None);
    }

    #[test]
    fn extracts_translation_metadata() {
        let md = "---\n\
            lang: zh-CN\n\
            translation-of: ./source.md\n\
            translator: did:plc:abc\n\
            tags:\n  - physics\n  - qm\n\
            ---\n\
            body\n";
        let meta = extract("source.zh-CN.md", md).unwrap();
        assert_eq!(meta.lang, "zh-CN");
        assert_eq!(meta.translation_of.as_deref(), Some("./source.md"));
        assert_eq!(meta.translator.as_deref(), Some("did:plc:abc"));
        assert_eq!(meta.tags, vec!["physics", "qm"]);
    }

    #[test]
    fn accepts_underscore_alias() {
        let md = "---\nlang: en\ntranslation_of: ./src.md\n---\n";
        let meta = extract("x.md", md).unwrap();
        assert_eq!(meta.translation_of.as_deref(), Some("./src.md"));
    }

    #[test]
    fn missing_frontmatter() {
        let md = "# Just content\n";
        assert!(matches!(
            extract("foo.md", md).unwrap_err(),
            ValidationError::MetadataMissing { .. }
        ));
    }

    #[test]
    fn missing_lang() {
        let md = "---\ntitle: No lang\n---\n";
        assert!(matches!(
            extract("foo.md", md).unwrap_err(),
            ValidationError::MissingLang { .. }
        ));
    }

    #[test]
    fn unterminated_fence() {
        let md = "---\nlang: en\n";
        assert!(matches!(
            extract("foo.md", md).unwrap_err(),
            ValidationError::MetadataParseError { .. }
        ));
    }

    #[test]
    fn handles_crlf() {
        let md = "---\r\nlang: en\r\n---\r\nbody\r\n";
        let meta = extract("x.md", md).unwrap();
        assert_eq!(meta.lang, "en");
    }
}
