use std::borrow::Cow;
use std::env::current_dir;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};
use log::{debug, warn};
use pijul_config::local::Local;
use pijul_core::DOT_DIR;

pub struct Repository {
    pub pristine: pijul_core::pristine::sanakirja::Pristine,
    pub changes: pijul_core::changestore::filesystem::FileSystem,
    pub working_copy: pijul_core::working_copy::filesystem::FileSystem,
    pub path: PathBuf,
    pub changes_dir: PathBuf,
}

pub const PRISTINE_DIR: &str = "pristine";
pub const CHANGES_DIR: &str = "changes";

#[cfg(unix)]
pub fn max_files() -> usize {
    let n = if let Ok((n, _)) = rlimit::getrlimit(rlimit::Resource::NOFILE) {
        let available_parallelism = match std::thread::available_parallelism() {
            Ok(available_parallelism) => available_parallelism.get(),
            Err(error) => {
                warn!("Failed to get available parallelism: {error}");
                1
            }
        };

        (n as usize / (2 * available_parallelism)).max(1)
    } else {
        256
    };

    debug!("max_files = {:?}", n);
    n
}

#[cfg(not(unix))]
pub fn max_files() -> usize {
    1
}

impl Repository {
    fn find_repository_root(
        directory_override: Option<&Path>,
        dot_dir: &str,
    ) -> Result<(PathBuf, PathBuf), anyhow::Error> {
        let starting_directory = match directory_override {
            Some(starting_directory) => starting_directory.to_path_buf(),
            None => std::env::current_dir()?,
        };

        let mut current_directory = starting_directory.as_path();
        while let Some(parent) = current_directory.parent() {
            let candidate_path = current_directory.join(dot_dir);
            debug!("Checking if `{candidate_path:?}` exists");

            if candidate_path.exists() && candidate_path.is_dir() {
                return Ok((current_directory.to_path_buf(), candidate_path));
            }

            current_directory = parent;
        }

        Err(anyhow!(
            "No Pijul repository found, starting from `{starting_directory:?}`"
        ))
    }

    pub fn find_root(cur: Option<&Path>) -> Result<Self, anyhow::Error> {
        let (working_copy_directory, dot_directory) = Self::find_repository_root(cur, DOT_DIR)?;
        let pristine_dir = dot_directory.join(PRISTINE_DIR);
        let changes_dir = dot_directory.join(CHANGES_DIR);

        Ok(Self {
            pristine: pijul_core::pristine::sanakirja::Pristine::new(&pristine_dir.join("db"))?,
            working_copy: pijul_core::working_copy::filesystem::FileSystem::from_root(
                &working_copy_directory,
            ),
            changes: pijul_core::changestore::filesystem::FileSystem::from_root(
                &working_copy_directory,
                max_files(),
            ),
            path: working_copy_directory,
            changes_dir,
        })
    }

    pub fn init(
        config: &pijul_config::Config,
        path: Option<&Path>,
        kind: Option<&str>,
        remote: Option<&str>,
    ) -> Result<Self, anyhow::Error> {
        let cur = if let Some(path) = path {
            Cow::Borrowed(path)
        } else {
            Cow::Owned(current_dir()?)
        };

        let pristine_dir = cur.join(DOT_DIR).join(PRISTINE_DIR);

        if std::fs::metadata(&pristine_dir).is_err() {
            std::fs::create_dir_all(&pristine_dir)?;
            let dot_ignore_path = cur.join(".ignore");

            // Initialize the `.ignore` file, if it doesn't already exist
            if !dot_ignore_path.exists() {
                let mut dot_ignore_file = File::create_new(&dot_ignore_path)?;
                let file_contents = config.dot_ignore_contents(kind)?;
                dot_ignore_file.write_all(file_contents.as_bytes())?;
            }

            // Initialize the local configuration file on disk
            let mut local_config = Local::new(&cur);
            local_config.default_remote = remote.map(str::to_string);
            local_config.write()?;

            let changes_dir = cur.join(DOT_DIR).join(CHANGES_DIR);

            let mut stderr = std::io::stderr();
            writeln!(stderr, "Repository created at {}", cur.to_string_lossy())?;

            Ok(Repository {
                pristine: pijul_core::pristine::sanakirja::Pristine::new(&pristine_dir.join("db"))?,
                working_copy: pijul_core::working_copy::filesystem::FileSystem::from_root(&cur),
                changes: pijul_core::changestore::filesystem::FileSystem::from_root(
                    &cur,
                    max_files(),
                ),
                path: cur.into_owned(),
                changes_dir,
            })
        } else {
            bail!("Already in a repository")
        }
    }
}
