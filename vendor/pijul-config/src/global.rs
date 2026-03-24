use crate::author::Author;
use crate::{CONFIG_FILE, Shared};

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Global {
    #[serde(skip)]
    source_file: Option<PathBuf>,

    #[serde(default)]
    pub author: Author,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub ignore_kinds: HashMap<String, Vec<String>>,

    #[serde(flatten)]
    pub shared_config: Shared,
}

impl Global {
    /// Select which configuration file to use
    pub fn config_file() -> Option<PathBuf> {
        // {config_directory}/config.toml
        crate::global_config_directory().map(|config_directory| config_directory.join(CONFIG_FILE))
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

// pub fn global_config_dir() -> Option<PathBuf> {
//     if let Ok(path) = std::env::var("PIJUL_CONFIG_DIR") {
//         let dir = std::path::PathBuf::from(path);
//         Some(dir)
//     } else if let Some(mut dir) = dirs_next::config_dir() {
//         dir.push(CONFIG_DIR);
//         Some(dir)
//     } else {
//         None
//     }
// }

// impl Global {
//     pub fn load() -> Result<(Global, Option<u64>), anyhow::Error> {
//         let res = None
//             .or_else(|| {
//                 let mut path = global_config_dir()?;
//                 path.push("config.toml");
//                 try_load_file(path)
//             })
//             .or_else(|| {
//                 // Read from `$HOME/.config/pijul` dir
//                 let mut path = dirs_next::home_dir()?;
//                 path.push(".config");
//                 path.push(CONFIG_DIR);
//                 path.push("config.toml");
//                 try_load_file(path)
//             })
//             .or_else(|| {
//                 // Read from `$HOME/.pijulconfig`
//                 let mut path = dirs_next::home_dir()?;
//                 path.push(GLOBAL_CONFIG_DIR);
//                 try_load_file(path)
//             });

//         let Some((file, path)) = res else {
//             return Ok((Global::default(), None));
//         };

//         let mut file = file.map_err(|e| {
//             anyhow!("Could not open configuration file at {}", path.display()).context(e)
//         })?;

//         let mut buf = String::new();

//         file.read_to_string(&mut buf).map_err(|e| {
//             anyhow!("Could not read configuration file at {}", path.display()).context(e)
//         })?;

//         debug!("buf = {:?}", buf);

//         let global: Global = toml::from_str(&buf).map_err(|e| {
//             anyhow!("Could not parse configuration file at {}", path.display()).context(e)
//         })?;

//         let metadata = file.metadata()?;
//         let file_age = metadata
//             .modified()?
//             .duration_since(std::time::SystemTime::UNIX_EPOCH)?
//             .as_secs();

//         Ok((global, Some(file_age)))
//     }
// }
