//! Validator for NightBoat article bundles.
//!
//! Walks a bundle's source tree, extracts `lang` / `translation_of`
//! metadata from each MD and Typst source file, and groups files into
//! logical articles (one source + N official translations). Also writes
//! metadata back into source files via [`inject`] so a bundle published to
//! PDS is self-describing — every DB-side field is recoverable by
//! re-parsing the bundle.

pub mod path;
pub mod extract;
pub mod inject;
pub mod scan;
mod validate;

pub use scan::scan_dir;
pub use validate::{validate, validate_files};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Md,
    Typst,
}

/// Raw metadata extracted from a single file's frontmatter / `#metadata` block.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FileMeta {
    /// POSIX path relative to repo root, normalized (no `./`, `..`, duplicate `/`).
    pub path: String,
    pub format: Format,
    pub lang: String,
    /// For translations: POSIX path of the source file, relative to repo root.
    /// (Validator resolves repo-relative form; authors write paths relative to the
    /// translation file's directory.)
    pub translation_of: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, rename = "abstract")]
    pub abstract_: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub translator: Option<String>,
    #[serde(default)]
    pub translation_notes: Option<String>,
    /// Article-level fields injected by the publish path (read back here for
    /// re-indexing the AppView from PDS).
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub cover: Option<String>,
    #[serde(default)]
    pub related: Vec<String>,
}

/// One language version of an article.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Localization {
    pub lang: String,
    pub file_path: String,
    pub title: Option<String>,
    #[serde(rename = "abstract")]
    pub abstract_: Option<String>,
    pub tags: Vec<String>,
    pub translator: Option<String>,
    pub translation_notes: Option<String>,
}

/// One logical article = source file + all its official translations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Article {
    pub source_path: String,
    pub source_lang: String,
    /// Includes the source as one entry (lang = source_lang).
    /// Sorted by `lang` for deterministic output.
    pub localizations: Vec<Localization>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticleSet {
    /// Sorted by `source_path` for deterministic output.
    pub articles: Vec<Article>,
}

#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ValidationError {
    #[error("{path}: missing `lang` field in metadata")]
    MissingLang { path: String },

    #[error("{path}: `lang` value {lang:?} is not a valid BCP 47 tag")]
    InvalidLang { path: String, lang: String },

    #[error("{path}: invalid path ({reason})")]
    InvalidPath { path: String, reason: String },

    #[error("{path}: failed to parse metadata: {reason}")]
    MetadataParseError { path: String, reason: String },

    #[error("{path}: no nightboat metadata found")]
    MetadataMissing { path: String },

    #[error("{translation}: translation_of points to {target:?} which does not exist in the repo")]
    TranslationTargetMissing { translation: String, target: String },

    #[error("{translation}: translation_of points to {intermediate:?} which is itself a translation (chains not allowed)")]
    TranslationChain {
        translation: String,
        intermediate: String,
    },

    #[error("article {source_path}: multiple files claim lang {lang:?} ({files:?})")]
    DuplicateLanguage {
        source_path: String,
        lang: String,
        files: Vec<String>,
    },
}

impl ValidationError {
    /// Path of the offending file, for grouping errors in UI.
    pub fn path(&self) -> &str {
        match self {
            Self::MissingLang { path }
            | Self::InvalidLang { path, .. }
            | Self::InvalidPath { path, .. }
            | Self::MetadataParseError { path, .. }
            | Self::MetadataMissing { path } => path,
            Self::TranslationTargetMissing { translation, .. }
            | Self::TranslationChain { translation, .. } => translation,
            Self::DuplicateLanguage { source_path, .. } => source_path,
        }
    }
}

/// Collected errors — validator always returns all problems at once rather than
/// failing on the first. Callers render them to the author.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
}

impl ValidationReport {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn push(&mut self, err: ValidationError) {
        self.errors.push(err);
    }
}

/// Map keyed by normalized repo-relative path.
pub(crate) type FileMap = BTreeMap<String, FileMeta>;
