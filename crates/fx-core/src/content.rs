//! Typed enums for content metadata that were previously bare strings.

use serde::{Deserialize, Serialize};

/// Content format — determines which renderer to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum ContentFormat {
    Typst,
    Markdown,
    Html,
    #[serde(rename = "tex")]
    #[sqlx(rename = "tex")]
    Tex,
}

impl ContentFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Typst => "typst",
            Self::Markdown => "markdown",
            Self::Html => "html",
            Self::Tex => "tex",
        }
    }

    pub fn file_extension(self) -> &'static str {
        match self {
            Self::Typst => "typ",
            Self::Markdown => "md",
            Self::Html => "html",
            Self::Tex => "tex",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "typ" => Some(Self::Typst),
            "md" => Some(Self::Markdown),
            "html" | "htm" => Some(Self::Html),
            "tex" | "latex" => Some(Self::Tex),
            _ => None,
        }
    }
}

impl std::str::FromStr for ContentFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "typst" => Ok(Self::Typst),
            "markdown" | "md" => Ok(Self::Markdown),
            "html" => Ok(Self::Html),
            "tex" | "latex" => Ok(Self::Tex),
            other => Err(format!("unknown content format: {other}")),
        }
    }
}

impl std::fmt::Display for ContentFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Content kind — article, question, or answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum ContentKind {
    Article,
    Question,
    Answer,
}

impl ContentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Article => "article",
            Self::Question => "question",
            Self::Answer => "answer",
        }
    }
}

impl std::str::FromStr for ContentKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "article" => Ok(Self::Article),
            "question" => Ok(Self::Question),
            "answer" => Ok(Self::Answer),
            other => Err(format!("unknown content kind: {other}")),
        }
    }
}

impl std::fmt::Display for ContentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Article category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum Category {
    General,
    Lecture,
    Paper,
    Review,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Lecture => "lecture",
            Self::Paper => "paper",
            Self::Review => "review",
        }
    }
}

impl Default for Category {
    fn default() -> Self {
        Self::General
    }
}

impl std::str::FromStr for Category {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "general" => Ok(Self::General),
            "lecture" => Ok(Self::Lecture),
            "paper" => Ok(Self::Paper),
            "review" => Ok(Self::Review),
            other => Err(format!("unknown category: {other}")),
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Prerequisite strength.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum PrereqType {
    Required,
    Recommended,
    Suggested,
}

impl PrereqType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Recommended => "recommended",
            Self::Suggested => "suggested",
        }
    }
}

impl std::fmt::Display for PrereqType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_format_roundtrip() {
        for fmt in [ContentFormat::Typst, ContentFormat::Markdown, ContentFormat::Html, ContentFormat::Tex] {
            let s = serde_json::to_string(&fmt).unwrap();
            let back: ContentFormat = serde_json::from_str(&s).unwrap();
            assert_eq!(fmt, back);
        }
    }

    #[test]
    fn content_format_from_str() {
        assert_eq!("typst".parse::<ContentFormat>().unwrap(), ContentFormat::Typst);
        assert_eq!("md".parse::<ContentFormat>().unwrap(), ContentFormat::Markdown);
        assert!("invalid".parse::<ContentFormat>().is_err());
    }

    #[test]
    fn content_format_extension() {
        assert_eq!(ContentFormat::Typst.file_extension(), "typ");
        assert_eq!(ContentFormat::from_extension("md"), Some(ContentFormat::Markdown));
    }

    #[test]
    fn content_kind_roundtrip() {
        let json = serde_json::to_string(&ContentKind::Question).unwrap();
        assert_eq!(json, "\"question\"");
        let back: ContentKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ContentKind::Question);
    }

    #[test]
    fn prereq_type_as_str() {
        assert_eq!(PrereqType::Required.as_str(), "required");
        assert_eq!(PrereqType::Recommended.as_str(), "recommended");
        assert_eq!(PrereqType::Suggested.as_str(), "suggested");
    }

    #[test]
    fn prereq_type_serde_roundtrip() {
        let json = serde_json::to_string(&PrereqType::Required).unwrap();
        assert_eq!(json, "\"required\"");
        let back: PrereqType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PrereqType::Required);
    }

    #[test]
    fn category_default() {
        assert_eq!(Category::default(), Category::General);
    }
}
