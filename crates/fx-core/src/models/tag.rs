use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::content::PrereqType;

// ---- DB row ----

/// A single per-language label row in `tag_labels`. Every label belongs
/// to a tag (the concept); labels in the same tag share taxonomy and
/// prereq edges. The `names` map is assembled from the tag's sibling
/// labels at query time (see `tag_label_map` in the schema).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct Tag {
    pub id: String,
    pub name: String,
    #[ts(type = "Record<string, string>")]
    pub names: sqlx::types::Json<HashMap<String, String>>,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    /// The tag (concept) this label belongs to.
    pub tag_id: String,
    /// Language of this particular label (ISO code).
    pub lang: String,
}

// ---- Request ----

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct CreateTag {
    pub id: String,
    pub name: String,
    pub names: Option<HashMap<String, String>>,
    pub description: Option<String>,
}

// ---- Shared ----

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/lib/generated/")]
pub struct ArticlePrereq {
    pub tag_id: String,
    pub prereq_type: PrereqType,
}
