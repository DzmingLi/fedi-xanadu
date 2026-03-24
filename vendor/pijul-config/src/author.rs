use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Author {
    #[serde(alias = "full_name", default, skip_serializing_if = "String::is_empty")]
    pub display_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub email: String,
    // These fields have been moved to per-identity remotes.toml, but we still
    // need to read them from old identity.toml / config.toml files for migration.
    #[serde(alias = "name", default, skip_serializing)]
    pub username: String,
    #[serde(default, skip_serializing)]
    pub origin: String,
    // This has been moved to identity::Config, but we should still be able to read the values
    #[serde(default, skip_serializing)]
    pub key_path: Option<PathBuf>,
}

impl Default for Author {
    fn default() -> Self {
        Self {
            email: String::new(),
            display_name: whoami::realname(),
            username: String::new(),
            origin: String::new(),
            key_path: None,
        }
    }
}
