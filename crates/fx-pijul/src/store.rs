use std::path::{Path, PathBuf};

use pijul_core::{
    Base32, MutTxnT, MutTxnTExt, TxnT, TxnTExt,
    RecordBuilder, Algorithm,
    changestore::ChangeStore,
};

/// Wrapper around pijul-core for managing article repositories.
///
/// Each article gets its own pijul repo under `base_path/node_id/`.
pub struct PijulStore {
    base_path: PathBuf,
}

impl PijulStore {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    pub fn repo_path(&self, node_id: &str) -> PathBuf {
        self.base_path.join(node_id)
    }

    /// Initialize a new pijul repo for an article.
    pub fn init_repo(&self, node_id: &str) -> anyhow::Result<PathBuf> {
        let path = self.repo_path(node_id);
        std::fs::create_dir_all(&path)?;

        let dot_dir = path.join(pijul_core::DOT_DIR);
        if dot_dir.exists() {
            tracing::debug!("pijul repo already exists at {}", path.display());
            return Ok(path);
        }

        // Create .pijul directory structure
        let pristine_dir = dot_dir.join("pristine");
        let changes_dir = dot_dir.join("changes");
        std::fs::create_dir_all(&pristine_dir)?;
        std::fs::create_dir_all(&changes_dir)?;

        // Write a minimal .ignore file
        std::fs::write(path.join(".ignore"), "*.html\n")?;

        // Initialize the pristine database and create main channel
        let pristine = pijul_core::pristine::sanakirja::Pristine::new(&pristine_dir.join("db"))?;
        let mut txn = pristine.mut_txn_begin()?;
        txn.open_or_create_channel(pijul_core::DEFAULT_CHANNEL)?;
        txn.commit()?;

        tracing::info!("initialized pijul repo at {}", path.display());
        Ok(path)
    }

    /// Record all working copy changes as a new pijul change.
    pub fn record(&self, node_id: &str, message: &str) -> anyhow::Result<Option<String>> {
        let path = self.repo_path(node_id);
        let repo = self.open_repo(&path)?;

        let txn = repo.pristine.arc_txn_begin()?;
        let channel_name = pijul_core::DEFAULT_CHANNEL;

        // Load channel
        let channel = {
            let t = txn.read();
            t.load_channel(channel_name)?
                .ok_or_else(|| anyhow::anyhow!("Channel {channel_name} not found"))?
        };

        // Apply root change if needed (first record)
        txn.write().apply_root_change_if_needed(&repo.changes, &channel, rand::rng())?;

        // Add all untracked files in working copy
        {
            let mut t = txn.write();
            for entry in std::fs::read_dir(&path)? {
                let entry = entry?;
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // Skip .pijul directory, .ignore, and html cache
                if name_str.starts_with('.') || name_str == "content.html" {
                    continue;
                }
                let rel_path = name_str.to_string();
                if !t.is_tracked(&rel_path).unwrap_or(false) {
                    let is_dir = entry.file_type()?.is_dir();
                    t.add(&rel_path, is_dir, 0)?;
                }
            }
        }

        // Record changes
        let mut builder = RecordBuilder::new();
        builder.record(
            txn.clone(),
            Algorithm::default(),
            false,
            &pijul_core::DEFAULT_SEPARATOR,
            channel.clone(),
            &repo.working_copy,
            &repo.changes,
            "",
            1,
        )?;
        let rec = builder.finish();

        if rec.actions.is_empty() {
            txn.commit()?;
            return Ok(None);
        }

        // Build and save the change
        let actions: Vec<_> = rec.actions.into_iter().map(|a| {
            let t = txn.read();
            a.globalize(&*t).unwrap()
        }).collect();

        let contents = std::sync::Arc::try_unwrap(rec.contents).unwrap().into_inner();
        let contents_hash = {
            let mut hasher = pijul_core::pristine::Hasher::default();
            hasher.update(&contents[..]);
            hasher.finish()
        };

        let mut change = pijul_core::change::LocalChange {
            offsets: pijul_core::change::Offsets::default(),
            hashed: pijul_core::change::Hashed {
                version: pijul_core::change::VERSION,
                contents_hash,
                changes: actions,
                metadata: Vec::new(),
                dependencies: Vec::new(),
                extra_known: Vec::new(),
                header: pijul_core::change::ChangeHeader {
                    message: message.to_string(),
                    ..Default::default()
                },
            },
            unhashed: None,
            contents,
        };

        let hash = repo.changes.save_change(&mut change, |_, _| Ok::<_, anyhow::Error>(()))?;

        // Apply the change to the channel
        {
            let mut t = txn.write();
            t.apply_local_change(&channel, &change, &hash, &rec.updatables)?;
        }

        txn.commit()?;

        let hash_str = hash.to_base32();
        tracing::info!("recorded change {} for {}: {message}", hash_str, node_id);
        Ok(Some(hash_str))
    }

    /// Fork an existing repo by copying the entire directory.
    pub fn fork(&self, source_node_id: &str, fork_node_id: &str) -> anyhow::Result<PathBuf> {
        let source = self.repo_path(source_node_id);
        let fork_path = self.repo_path(fork_node_id);

        if !source.exists() {
            anyhow::bail!("source repo does not exist: {}", source.display());
        }

        copy_dir_recursive(&source, &fork_path)?;
        tracing::info!("forked {} -> {}", source.display(), fork_path.display());
        Ok(fork_path)
    }

    /// Get the list of change hashes in a repo's main channel.
    pub fn log(&self, node_id: &str) -> anyhow::Result<Vec<String>> {
        let path = self.repo_path(node_id);
        let repo = self.open_repo(&path)?;
        let txn = repo.pristine.txn_begin()?;
        let channel = txn.load_channel(pijul_core::DEFAULT_CHANNEL)?
            .ok_or_else(|| anyhow::anyhow!("No main channel"))?;

        let mut hashes = Vec::new();
        let ch = channel.read();
        for entry in txn.log(&*ch, 0)? {
            let (_, (hash, _)) = entry?;
            let h: pijul_core::Hash = hash.into();
            hashes.push(h.to_base32());
        }
        Ok(hashes)
    }

    fn open_repo(&self, path: &Path) -> anyhow::Result<RepoHandle> {
        let dot_dir = path.join(pijul_core::DOT_DIR);
        let pristine_dir = dot_dir.join("pristine");

        Ok(RepoHandle {
            pristine: pijul_core::pristine::sanakirja::Pristine::new(&pristine_dir.join("db"))?,
            changes: pijul_core::changestore::filesystem::FileSystem::from_root(
                path,
                pijul_repository::max_files(),
            ),
            working_copy: pijul_core::working_copy::filesystem::FileSystem::from_root(path),
        })
    }
}

struct RepoHandle {
    pristine: pijul_core::pristine::sanakirja::Pristine,
    changes: pijul_core::changestore::filesystem::FileSystem,
    working_copy: pijul_core::working_copy::filesystem::FileSystem,
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}
