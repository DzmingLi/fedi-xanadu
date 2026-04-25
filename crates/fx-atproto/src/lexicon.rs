/// AT Protocol Lexicon NSID constants for NightBoat.
///
/// NSIDs live under `at.nightbo.*` (reverse DNS of nightbo.at). This prefix is
/// **permanent** — it is baked into every PDS record a user writes, so it must
/// outlive any deploy-domain changes. Do not retie this to the current host.
pub const WORK: &str = "at.nightbo.work";
/// Deprecated alias kept temporarily for code that hasn't migrated.
/// New code should use `WORK`.
#[deprecated(note = "use WORK")]
pub const ARTICLE: &str = "at.nightbo.work";
pub const FORK: &str = "at.nightbo.fork";
pub const MERGE: &str = "at.nightbo.merge";
pub const VOTE: &str = "at.nightbo.vote";
pub const COMMENT: &str = "at.nightbo.comment";
pub const LEARNED: &str = "at.nightbo.learned";
pub const BOOKMARK: &str = "at.nightbo.bookmark";
pub const BOOK_RATING: &str = "at.nightbo.book.rating";
pub const BOOK_READING_STATUS: &str = "at.nightbo.book.readingstatus";
pub const BOOK_SHORT_REVIEW: &str = "at.nightbo.book.shortReview";
pub const BOOK_SERIES_RATING: &str = "at.nightbo.bookseries.rating";
pub const BOOK_SERIES_SHORT_REVIEW: &str = "at.nightbo.bookseries.shortReview";
pub const COURSE_RATING: &str = "at.nightbo.course.rating";
/// Deprecated: series is no longer a separate NSID — at.nightbo.work covers
/// both single-entry and multi-chapter works. Kept temporarily for migration
/// code that needs to detect legacy series records.
#[deprecated(note = "series and article unified under WORK")]
pub const SERIES: &str = "at.nightbo.work";
pub const SKILL: &str = "at.nightbo.skill";
pub const WANT_LEARN: &str = "at.nightbo.wantlearn";
pub const TAG: &str = "at.nightbo.tag";
pub const TAG_REL: &str = "at.nightbo.tagrel";
pub const DRAFT: &str = "at.nightbo.draft";
pub const DEFS: &str = "at.nightbo.defs";
pub const AUTHORSHIP: &str = "at.nightbo.authorship";
pub const ORCID: &str = "at.nightbo.orcid";
pub const SKILL_TREE: &str = "at.nightbo.skilltree";
pub const PIJUL_REF_UPDATE: &str = "sh.tangled.pijul.refUpdate";
pub const PUBLICATION: &str = "at.nightbo.publication";
pub const PUBLICATION_MEMBERSHIP: &str = "at.nightbo.publication.membership";
pub const PUBLICATION_ENTRY: &str = "at.nightbo.publication.entry";
pub const PUBLICATION_FOLLOW: &str = "at.nightbo.publication.follow";
