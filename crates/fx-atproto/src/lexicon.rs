/// AT Protocol Lexicon NSID constants for NightBoat.
///
/// NSIDs live under `at.nightbo.*` (reverse DNS of nightbo.at). This prefix is
/// **permanent** — it is baked into every PDS record a user writes, so it must
/// outlive any deploy-domain changes. Do not retie this to the current host.
pub const ARTICLE: &str = "at.nightbo.article";
pub const FORK: &str = "at.nightbo.fork";
pub const MERGE: &str = "at.nightbo.merge";
pub const VOTE: &str = "at.nightbo.vote";
pub const COMMENT: &str = "at.nightbo.comment";
pub const LEARNED: &str = "at.nightbo.learned";
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
