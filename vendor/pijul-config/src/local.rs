use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::remote::RemoteConfig;
use crate::{CONFIG_FILE, REPOSITORY_CONFIG_FILE, Shared};

use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Local {
    #[serde(skip)]
    source_file: Option<PathBuf>,

    pub default_remote: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remotes: Vec<RemoteConfig>,

    #[serde(flatten)]
    pub shared_config: Shared,
}

impl Local {
    pub fn new(repository_path: &Path) -> Self {
        let config_path = Self::config_file(repository_path);
        Self {
            source_file: Some(config_path),
            ..Self::default()
        }
    }

    pub fn config_file(repository_path: &Path) -> PathBuf {
        let dot_directory = repository_path.join(pijul_core::DOT_DIR);
        let old_config_path = dot_directory.join(REPOSITORY_CONFIG_FILE);

        match old_config_path.exists() {
            true => old_config_path,
            false => dot_directory.join(CONFIG_FILE),
        }
    }

    pub fn read_contents(config_path: &Path) -> Result<String, anyhow::Error> {
        let mut config_file = File::open(config_path)?;
        let mut file_contents = String::new();
        config_file.read_to_string(&mut file_contents)?;

        Ok(file_contents)
    }

    pub fn parse_contents(config_path: &Path, toml_data: &str) -> Result<Self, anyhow::Error> {
        let mut config: Self = toml::from_str(&toml_data)?;

        // Store the location of the original configuration file, so it can later be written to
        // The `source_file` field is annotated with `#[serde(skip)]` and should be always be None unless set manually
        assert!(config.source_file.is_none());
        config.source_file = Some(config_path.to_path_buf());

        Ok(config)
    }

    pub fn write(&self) -> Result<(), anyhow::Error> {
        let mut config_file = File::create(self.source_file.as_ref().unwrap())?;
        let file_contents = toml::to_string_pretty(self)?;
        config_file.write_all(file_contents.as_bytes())?;

        Ok(())
    }
}
