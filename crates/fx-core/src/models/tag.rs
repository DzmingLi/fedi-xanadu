use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::content::PrereqType;

// ---- DB row ----

/// One name in some language for a tag (concept). Every name row
/// belongs to a `tag_id`; a single tag may have many names across
/// languages (or multiple synonyms within one language — "ML" and
/// "Machine Learning" both attach to the same `tag_id`). No name is
/// privileged as "primary" — which one to show is a viewer preference
/// (see `user_name_pref`) with a default of earliest-added in locale.
///
/// `names` is a derived field populated by `tag_label_map(tag_id)` at
/// query time: a `{lang → earliest-added-name}` map used by the
/// frontend to display the same concept in different languages
/// without a round trip.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Tag {
    /// Row id of the name itself (`tn-…`).
    pub id: String,
    /// The display string, in `lang`.
    pub name: String,
    /// Derived at query time: `{lang → earliest-added name}` for every
    /// language this tag has a name in.
    #[ts(type = "Record<string, string>")]
    pub names: sqlx::types::Json<HashMap<String, String>>,
    /// When this specific name row was added. Used for the
    /// earliest-added-wins default display rule.
    pub added_at: DateTime<Utc>,
    /// The concept this name belongs to (`tg-…`).
    pub tag_id: String,
    /// Language of this specific name (ISO code).
    pub lang: String,
}

// ---- Request ----

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateTag {
    /// The initial name in its own language.
    pub name: String,
    /// Optional extra names keyed by language. Each entry becomes its
    /// own `tag_names` row attached to the same tag.
    pub names: Option<HashMap<String, String>>,
}

// ---- Shared ----

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ArticlePrereq {
    pub tag_id: String,
    pub prereq_type: PrereqType,
}
