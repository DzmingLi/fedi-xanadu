//! Inject NightBoat metadata back into source files.
//!
//! Counterpart to [`crate::extract`]: extract reads format-native metadata
//! slots; this module writes them. Used by the publish path so the bundle
//! stored on the user's PDS is self-describing — every DB-side metadata
//! field is recoverable by parsing the bundle's source files.
//!
//! Format-specific submodules:
//! - [`typst`] — `#metadata((...)) <nbt-article>` directive (typst AST)
//! - [`md`] — YAML frontmatter (the canonical writer lives in `fx-core::meta`
//!   to avoid circular deps; this module re-exports the field shape)
//!
//! HTML is intentionally NOT injected into the source — the bundle carries
//! a sibling `meta.json` instead, written by the publish path directly.

pub mod typst;

/// Label for article-level typst metadata. Article-scope: title/abstract/
/// tags/license/cover/... — describes ONE article (a standalone work or a
/// single chapter inside a series).
pub const TYPST_ARTICLE_LABEL: &str = "nbt-article";

/// Label for series-level typst metadata. Lives in a series's `main.typ`
/// and describes the whole multi-chapter work: title/description/lang/
/// category/topics. Per the `at.nightbo.work` lexicon, typst series carry
/// their series-level metadata here instead of a `meta.yml` sibling.
pub const TYPST_SERIES_LABEL: &str = "nbt-series";

/// Article-level metadata payload, format-agnostic. Each field is optional;
/// the writer skips fields that are `None`/empty so partial updates don't
/// blow away keys it can't see.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    pub title: Option<String>,
    pub abstract_: Option<String>,
    pub lang: Option<String>,
    pub category: Option<String>,
    pub license: Option<String>,
    pub cover: Option<String>,
    pub tags: Vec<String>,
    pub related: Vec<String>,
}
