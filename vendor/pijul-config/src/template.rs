use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Template {
    pub message: Option<PathBuf>,
    pub description: Option<PathBuf>,
}
