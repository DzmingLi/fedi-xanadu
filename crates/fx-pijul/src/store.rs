use std::path::{Path, PathBuf};

use pijul_core::{
    Base32, MutTxnT, MutTxnTExt, TxnT, TxnTExt, TreeTxnT,
    RecordBuilder, Algorithm,
    changestore::ChangeStore,
    working_copy::WorkingCopyRead,
};

/// A single line-level difference between the working copy and the last recorded state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// The file path relative to the repo root.
    pub path: String,
    /// Lines removed from the recorded state (prefixed with context of where).
    pub removed: Vec<String>,
    /// Lines added in the working copy.
    pub added: Vec<String>,
}

/// Summary of differences between the working copy and the last recorded state.
#[derive(Debug, Clone, Default)]
pub struct DiffResult {
    /// Per-file hunks showing what changed.
    pub hunks: Vec<DiffHunk>,
    /// Files that exist in the working copy but are not yet tracked.
    pub untracked: Vec<String>,
}

/// Metadata about a tracked file in a repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedFile {
    /// The file path relative to the repo root.
    pub path: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
}

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

    /// Show what changed between the working copy and the last recorded state.
    ///
    /// For each tracked file, compares the working copy content against the pristine
    /// (last-recorded) content and returns structured diff hunks. Also reports untracked
    /// files that exist in the working directory but have not been added.
    pub fn diff(&self, node_id: &str) -> anyhow::Result<DiffResult> {
        let path = self.repo_path(node_id);
        let repo = self.open_repo(&path)?;
        let txn = repo.pristine.arc_txn_begin()?;
        let channel_name = pijul_core::DEFAULT_CHANNEL;

        let channel = {
            let t = txn.read();
            t.load_channel(channel_name)?
                .ok_or_else(|| anyhow::anyhow!("Channel {channel_name} not found"))?
        };

        let mut result = DiffResult::default();

        // Collect tracked files from the pristine
        let tracked: Vec<(String, bool)> = {
            let t = txn.read();
            let mut files = Vec::new();
            for entry in t.iter_working_copy() {
                let (_inode, name, is_dir) = entry?;
                files.push((name, is_dir));
            }
            files
        };

        // For each tracked file, get its pristine content and compare with working copy
        for (file_path, is_dir) in &tracked {
            if *is_dir {
                continue;
            }

            // Read working copy content
            let wc_content = match repo.working_copy.read_file(file_path, &mut Vec::new()) {
                Ok(()) => {
                    let mut buf = Vec::new();
                    repo.working_copy.read_file(file_path, &mut buf)?;
                    buf
                }
                Err(_) => {
                    // File was deleted from working copy
                    continue;
                }
            };

            // Get pristine content by outputting from the graph
            let pristine_content = {
                let t = txn.read();
                let ch = channel.read();
                // Find the file's position in the graph
                match pijul_core::fs::find_inode(&*t, file_path) {
                    Ok(inode) => {
                        match t.get_inodes(&inode, None) {
                            Ok(Some(&pos)) => {
                                drop(ch);
                                drop(t);
                                let mut buf = Vec::new();
                                let mut out = pijul_core::vertex_buffer::Writer::new(&mut buf);
                                match pijul_core::output::output_file(
                                    &repo.changes, &txn, &channel, pos, &mut out,
                                ) {
                                    Ok(()) => Some(buf),
                                    Err(_) => None,
                                }
                            }
                            _ => None,
                        }
                    }
                    Err(_) => None,
                }
            };

            let pristine_str = pristine_content
                .as_ref()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            let wc_str = String::from_utf8_lossy(&wc_content).into_owned();

            if pristine_str != wc_str {
                let old_lines: Vec<&str> = pristine_str.lines().collect();
                let new_lines: Vec<&str> = wc_str.lines().collect();

                let removed: Vec<String> = old_lines
                    .iter()
                    .filter(|l| !new_lines.contains(l))
                    .map(|l| l.to_string())
                    .collect();
                let added: Vec<String> = new_lines
                    .iter()
                    .filter(|l| !old_lines.contains(l))
                    .map(|l| l.to_string())
                    .collect();

                if !removed.is_empty() || !added.is_empty() {
                    result.hunks.push(DiffHunk {
                        path: file_path.clone(),
                        removed,
                        added,
                    });
                }
            }
        }

        // Find untracked files
        let tracked_names: std::collections::HashSet<&str> =
            tracked.iter().map(|(n, _)| n.as_str()).collect();
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_string();
            if name_str.starts_with('.') || name_str == "content.html" {
                continue;
            }
            if !tracked_names.contains(name_str.as_str()) {
                result.untracked.push(name_str);
            }
        }

        txn.commit()?;
        Ok(result)
    }

    /// Apply a recorded change (by hash) from a source repo into a target repo.
    ///
    /// This copies the change file from the source repo's change store into the
    /// target repo's change store, then applies it to the target's main channel.
    /// After applying, the working copy is updated via `output_repository_no_pending`.
    ///
    /// This is a simplified form of collaborative merging: it works well for
    /// non-conflicting changes. For conflicting changes, pijul will insert
    /// conflict markers.
    pub fn apply(
        &self,
        source_node_id: &str,
        target_node_id: &str,
        change_hash: &str,
    ) -> anyhow::Result<()> {
        let hash = pijul_core::Hash::from_base32(change_hash.as_bytes())
            .ok_or_else(|| anyhow::anyhow!("Invalid change hash: {change_hash}"))?;

        let source_path = self.repo_path(source_node_id);
        let target_path = self.repo_path(target_node_id);

        if !source_path.exists() {
            anyhow::bail!("source repo does not exist: {}", source_path.display());
        }
        if !target_path.exists() {
            anyhow::bail!("target repo does not exist: {}", target_path.display());
        }

        // Copy the change file from source to target change store.
        // pijul stores changes at `.pijul/changes/XX/REST.change` where XX is
        // the first two base32 chars of the hash.
        let source_repo = self.open_repo(&source_path)?;
        let source_change_file = source_repo.changes.filename(&hash);
        let target_repo_tmp = self.open_repo(&target_path)?;
        let target_change_file = target_repo_tmp.changes.filename(&hash);

        if !source_change_file.exists() {
            anyhow::bail!(
                "change file not found in source: {}",
                source_change_file.display()
            );
        }

        if let Some(parent) = target_change_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !target_change_file.exists() {
            std::fs::copy(&source_change_file, &target_change_file)?;
        }

        // Also copy any dependency change files that aren't in the target yet
        let change = source_repo.changes.get_change(&hash)?;
        for dep_hash in &change.hashed.dependencies {
            let dep_src = source_repo.changes.filename(dep_hash);
            let dep_dst = target_repo_tmp.changes.filename(dep_hash);
            if dep_src.exists() && !dep_dst.exists() {
                if let Some(parent) = dep_dst.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&dep_src, &dep_dst)?;
            }
        }

        // Drop temporary handles before reopening
        drop(source_repo);
        drop(target_repo_tmp);

        // Open target repo and apply the change
        let repo = self.open_repo(&target_path)?;
        let txn = repo.pristine.arc_txn_begin()?;

        let channel = {
            let t = txn.read();
            t.load_channel(pijul_core::DEFAULT_CHANNEL)?
                .ok_or_else(|| anyhow::anyhow!("No main channel in target repo"))?
        };

        // Check if this change is already applied
        let already_applied = {
            let t = txn.read();
            t.has_change(&channel, &hash)?.is_some()
        };
        if already_applied {
            txn.commit()?;
            tracing::debug!("change {} already applied to {}", change_hash, target_node_id);
            return Ok(());
        }

        // Apply the change (and its dependencies recursively)
        {
            let mut t = txn.write();
            let mut ch = channel.write();
            t.apply_change_rec(&repo.changes, &mut *ch, &hash)?;
        }

        // Output the updated pristine to the working copy
        pijul_core::output::output_repository_no_pending(
            &repo.working_copy,
            &repo.changes,
            &txn,
            &channel,
            "",
            true,
            None,
            1,
            0,
        ).map_err(|e| anyhow::anyhow!("Failed to output working copy: {:?}", e))?;

        txn.commit()?;

        tracing::info!(
            "applied change {} from {} to {}",
            change_hash,
            source_node_id,
            target_node_id
        );
        Ok(())
    }

    /// Revert the working copy to the last recorded state.
    ///
    /// This overwrites all files in the working copy with their pristine
    /// (last-recorded) versions, discarding any unrecorded modifications.
    /// Untracked files are left untouched.
    pub fn revert(&self, node_id: &str) -> anyhow::Result<()> {
        let path = self.repo_path(node_id);
        let repo = self.open_repo(&path)?;
        let txn = repo.pristine.arc_txn_begin()?;

        let channel = {
            let t = txn.read();
            t.load_channel(pijul_core::DEFAULT_CHANNEL)?
                .ok_or_else(|| anyhow::anyhow!("No main channel"))?
        };

        // Output the pristine state to the working copy, overwriting modifications
        pijul_core::output::output_repository_no_pending(
            &repo.working_copy,
            &repo.changes,
            &txn,
            &channel,
            "",
            true,
            None,
            1,
            0,
        ).map_err(|e| anyhow::anyhow!("Failed to revert working copy: {:?}", e))?;

        txn.commit()?;
        tracing::info!("reverted working copy for {}", node_id);
        Ok(())
    }

    /// Read the current content of a file from the working copy.
    ///
    /// The `file_name` is relative to the repo root (e.g. `"content.typ"`).
    /// Returns the raw bytes of the file.
    pub fn get_file_content(&self, node_id: &str, file_name: &str) -> anyhow::Result<Vec<u8>> {
        let path = self.repo_path(node_id);
        let file_path = path.join(file_name);

        if !file_path.exists() {
            anyhow::bail!(
                "file not found: {} in repo {}",
                file_name,
                node_id
            );
        }

        // Ensure the resolved path is still inside the repo (prevent path traversal)
        let canonical = file_path.canonicalize()?;
        let repo_canonical = path.canonicalize()?;
        if !canonical.starts_with(&repo_canonical) {
            anyhow::bail!("path traversal detected: {}", file_name);
        }

        let content = std::fs::read(&file_path)?;
        Ok(content)
    }

    /// List all tracked files in a repository.
    ///
    /// Returns a list of `TrackedFile` entries with their relative paths
    /// and whether each entry is a directory.
    pub fn list_files(&self, node_id: &str) -> anyhow::Result<Vec<TrackedFile>> {
        let path = self.repo_path(node_id);
        let repo = self.open_repo(&path)?;
        let txn = repo.pristine.txn_begin()?;

        let mut files = Vec::new();
        for entry in txn.iter_working_copy() {
            let (_inode, name, is_dir) = entry?;
            files.push(TrackedFile {
                path: name,
                is_dir,
            });
        }

        Ok(files)
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
