use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RemoteConfig {
    Ssh {
        name: String,
        ssh: String,
    },
    Http {
        name: String,
        http: String,
        #[serde(default)]
        headers: HashMap<String, RemoteHttpHeader>,
    },
}

impl RemoteConfig {
    pub fn name(&self) -> &str {
        match self {
            RemoteConfig::Ssh { name, .. } => name,
            RemoteConfig::Http { name, .. } => name,
        }
    }

    pub fn url(&self) -> &str {
        match self {
            RemoteConfig::Ssh { ssh, .. } => ssh,
            RemoteConfig::Http { http, .. } => http,
        }
    }

    pub fn db_uses_name(&self) -> bool {
        match self {
            RemoteConfig::Ssh { .. } => false,
            RemoteConfig::Http { .. } => true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RemoteHttpHeader {
    String(String),
    Shell(Shell),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shell {
    pub shell: String,
}
