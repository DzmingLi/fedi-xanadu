pub mod author;
pub mod global;
pub mod hook;
pub mod local;
pub mod remote;
pub mod template;

use author::Author;
use global::Global;
use hook::Hooks;
use local::Local;
use remote::RemoteConfig;
use template::Template;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use dialoguer::theme;
use figment::Figment;
use figment::providers::{Format, Toml};
use log::{info, warn};
use serde_derive::{Deserialize, Serialize};

pub const DEFAULT_CONFIG: &str = include_str!("defaults.toml");
pub const REPOSITORY_CONFIG_FILE: &str = "config";
pub const GLOBAL_CONFIG_FILE: &str = ".pijulconfig";
pub const CONFIG_DIR: &str = "pijul";
pub const CONFIG_FILE: &str = "config.toml";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Shared {
    pub unrecord_changes: Option<usize>,
    pub reset_overwrites_changes: Option<Choice>,
    pub colors: Option<Choice>,
    pub pager: Option<Choice>,
    pub template: Option<Template>,
    #[serde(default)]
    pub hooks: Hooks,
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    // Store a copy of the original files, so that they can be modified independently
    #[serde(skip)]
    global_config: Option<Global>,
    #[serde(skip)]
    local_config: Option<Local>,

    // Global
    #[serde(default)]
    pub author: Author,
    pub ignore_kinds: HashMap<String, Vec<String>>,

    // Local
    pub default_remote: Option<String>,
    pub extra_dependencies: Vec<String>,
    pub remotes: Vec<RemoteConfig>,

    // Shared
    pub unrecord_changes: Option<usize>,
    pub reset_overwrites_changes: Choice,
    pub colors: Choice,
    pub pager: Choice,
    pub template: Option<Template>,
    #[serde(default)]
    pub hooks: hook::Hooks,
}

impl Config {
    pub fn load(
        repository_path: Option<&Path>,
        config_overrides: Vec<(String, String)>,
    ) -> Result<Self, anyhow::Error> {
        let global_config = match Global::config_file() {
            Some(global_config_path) => match Global::read_contents(&global_config_path) {
                Ok(contents) => Some((global_config_path, contents)),
                Err(error) => {
                    warn!("Unable to read global config file: {error:#?}");
                    None
                }
            },
            None => {
                warn!("Unable to find global configuration path");
                None
            }
        };

        let local_config = match repository_path {
            Some(repository_path) => match Local::read_contents(&repository_path) {
                Ok(contents) => Some((repository_path.to_path_buf(), contents)),
                Err(error) => {
                    warn!("Unable to read global config file: {error:#?}");
                    None
                }
            },
            None => {
                info!(
                    "Skipping local configuration path - repository path was not supplied by caller"
                );
                None
            }
        };

        Self::load_with(global_config, local_config, config_overrides)
    }

    pub fn load_with(
        global_config_file: Option<(PathBuf, String)>,
        local_config_file: Option<(PathBuf, String)>,
        config_overrides: Vec<(String, String)>,
    ) -> Result<Self, anyhow::Error> {
        // Merge the two configuration values, using the raw TOML string instead of the deserialized structs.
        // Figment uses a dictionary to store which fields are set, and using an already-deserialized
        // struct will guarantee that each layer will override the previous one.
        //
        // For example, if the optional `unrecord_changes` field is set as 1 globally but not set locally:
        // - Using deserialized structs (incorrect behaviour):
        //      - Global config is set to Some(1)
        //      - Local config is set to None - no value was found, so serde inserted the default
        //      - The local config technically has a value set, so the final (incorrect) value is None
        // - Using strings (correct behaviour):
        //      - Global config is set to Some(1)
        //      - Local config is unset
        //      - The final (correct) value is Some(1)
        let mut layers = Figment::new();

        // 1. Included defaults (defaults.toml)
        layers = layers.merge(Toml::string(DEFAULT_CONFIG));

        // 2. Global config
        let global_config = match global_config_file {
            Some((path, contents)) => {
                // Parse the config (and make sure it's valid!)
                let global_config = Global::parse_contents(&path, &contents)?;
                // Add the configuration layer as a string
                layers = layers.merge(Toml::string(&contents));

                Some(global_config)
            }
            None => None,
        };

        // 3. Local config
        let local_config = match local_config_file {
            Some((path, contents)) => {
                // Parse the config (and make sure it's valid!)
                let global_config = Local::parse_contents(&path, &contents)?;
                // Add the configuration layer as a string
                layers = layers.merge(Toml::string(&contents));

                Some(global_config)
            }
            None => None,
        };

        // 4. Command-line configuration overrides
        for (key, value) in config_overrides {
            layers = layers.join((key, value));
        }

        // Extract the configuration
        let mut config: Self = layers.extract()?;

        // These fields are annotated with #[serde(skip)] and therefore should be None
        assert!(config.global_config.is_none());
        assert!(config.local_config.is_none());

        // Store the original configuration sources so they can be modified later
        config.global_config = global_config;
        config.local_config = local_config;

        Ok(config)
    }

    pub fn global(&self) -> Option<Global> {
        self.global_config.clone()
    }

    pub fn local(&self) -> Option<Local> {
        self.local_config.clone()
    }

    pub fn dot_ignore_contents(&self, ignore_kind: Option<&str>) -> Result<String, anyhow::Error> {
        // The default entry is guaranteed to be present
        let default_ignore_lines = self.ignore_kinds.get("default").unwrap();

        // Find any extra lines to add to the `.ignore`, if they exist
        let extra_ignore_lines = match ignore_kind {
            Some(kind) => match self.ignore_kinds.get(kind) {
                Some(extra_ignore_lines) => extra_ignore_lines.iter(),
                None => {
                    return Err(anyhow::anyhow!(
                        "Unable to find specific ignore kind: {kind}"
                    ));
                }
            },
            None => [].iter(),
        };

        // Merge the default and specific ignore lines
        let mut ignore_lines = default_ignore_lines
            .iter()
            .chain(extra_ignore_lines)
            .map(|line| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // Add a newline at the end of the file
        if !ignore_lines.is_empty() {
            ignore_lines.push('\n');
        }

        Ok(ignore_lines)
    }

    /// Choose the right dialoguer theme based on user's config
    pub fn theme(&self) -> Box<dyn theme::Theme + Send + Sync> {
        match self.colors {
            Choice::Auto | Choice::Always => Box::new(theme::ColorfulTheme::default()),
            Choice::Never => Box::new(theme::SimpleTheme),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Choice {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "never")]
    Never,
}

/// Select which configuration directory to use
pub fn global_config_directory() -> Option<PathBuf> {
    // 1. $PIJUL_CONFIG_DIR/
    std::env::var("PIJUL_CONFIG_DIR")
        .ok()
        .map(PathBuf::from)
        .map(|directory| directory.join(CONFIG_FILE))
        // 2. ~/.config/pijul/
        .or_else(|| match dirs_next::config_dir() {
            Some(global_config_dir) => Some(global_config_dir.join(CONFIG_DIR)),
            None => None,
        })
        // 3. ~/.pijulconfig/
        .or_else(|| match dirs_next::home_dir() {
            Some(home_dir) => Some(home_dir.join(CONFIG_DIR)),
            None => None,
        })
}

/// Parse a command-line configuration argument into a key/value pair
pub fn parse_config_arg(argument: &str) -> Result<(String, String), anyhow::Error> {
    let (key, value) = argument
        .split_once('=')
        .ok_or(anyhow::anyhow!("Unable to find '=' character"))?;

    Ok((key.to_string(), value.to_string()))
}
