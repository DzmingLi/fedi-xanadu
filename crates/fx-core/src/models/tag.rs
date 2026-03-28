use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::content::PrereqType;

// ---- DB row ----

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub names: sqlx::types::Json<HashMap<String, String>>,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

// ---- Request ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTag {
    pub id: String,
    pub name: String,
    pub names: Option<HashMap<String, String>>,
    pub description: Option<String>,
}

// ---- Shared ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticlePrereq {
    pub tag_id: String,
    pub prereq_type: PrereqType,
}
