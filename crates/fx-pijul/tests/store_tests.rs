use fx_pijul::PijulStore;
use tempfile::TempDir;

fn setup() -> (TempDir, PijulStore) {
    let tmp = TempDir::new().expect("failed to create temp dir");
    let store = PijulStore::new(tmp.path());
    (tmp, store)
}

fn write_file(store: &PijulStore, node_id: &str, name: &str, content: &str) {
    let path = store.repo_path(node_id).join(name);
    std::fs::write(&path, content).expect("failed to write file");
}

// ---------------------------------------------------------------------------
// init_repo
// ---------------------------------------------------------------------------

#[test]
fn test_init_repo_creates_directory_structure() {
    let (_tmp, store) = setup();
    let path = store.init_repo("repo1").unwrap();

    assert!(path.exists());
    assert!(path.join(".pijul").exists());
    assert!(path.join(".pijul/pristine").exists());
    assert!(path.join(".pijul/changes").exists());
    assert!(path.join(".ignore").exists());

    let ignore = std::fs::read_to_string(path.join(".ignore")).unwrap();
    assert!(ignore.contains("*.html"));
}

#[test]
fn test_init_repo_idempotent() {
    let (_tmp, store) = setup();

    let path1 = store.init_repo("repo1").unwrap();
    // Write a file so we can verify the repo isn't wiped on reinit
    write_file(&store, "repo1", "content.typ", "keep me\n");
    store.record("repo1", "initial").unwrap();

    let path2 = store.init_repo("repo1").unwrap();
    assert_eq!(path1, path2);

    // Content should still be there after reinit
    let content = store.get_file_content("repo1", "content.typ").unwrap();
    assert_eq!(content, b"keep me\n");
}

#[test]
fn test_init_repo_returns_correct_path() {
    let (tmp, store) = setup();
    let path = store.init_repo("my-article").unwrap();
    assert_eq!(path, tmp.path().join("my-article"));
}

// ---------------------------------------------------------------------------
// record
// ---------------------------------------------------------------------------

#[test]
fn test_record_returns_hash() {
    let (_tmp, store) = setup();
    store.init_repo("r").unwrap();

    write_file(&store, "r", "content.typ", "hello\n");
    let hash = store.record("r", "first").unwrap();
    assert!(hash.is_some(), "first record should produce a change hash");
    assert!(!hash.as_ref().unwrap().is_empty());
}

#[test]
fn test_record_no_changes_returns_none() {
    let (_tmp, store) = setup();
    store.init_repo("r").unwrap();

    write_file(&store, "r", "content.typ", "hello\n");
    store.record("r", "first").unwrap();

    // Recording again with no changes should return None
    let hash = store.record("r", "no-op").unwrap();
    assert!(hash.is_none(), "recording with no changes should return None");
}

#[test]
fn test_record_multiple_changes() {
    let (_tmp, store) = setup();
    store.init_repo("r").unwrap();

    write_file(&store, "r", "content.typ", "v1\n");
    let h1 = store.record("r", "version 1").unwrap().expect("should record");

    write_file(&store, "r", "content.typ", "v2\n");
    let h2 = store.record("r", "version 2").unwrap().expect("should record");

    assert_ne!(h1, h2, "different changes should have different hashes");
}

#[test]
fn test_record_adds_new_files_automatically() {
    let (_tmp, store) = setup();
    store.init_repo("r").unwrap();

    write_file(&store, "r", "content.typ", "main\n");
    store.record("r", "first").unwrap();

    // Add a second file — record should auto-track it
    write_file(&store, "r", "extra.txt", "extra\n");
    let hash = store.record("r", "add extra").unwrap();
    assert!(hash.is_some(), "adding a new file should produce a change");

    let files = store.list_files("r").unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"extra.txt"));
}

#[test]
fn test_record_skips_dotfiles_and_html() {
    let (_tmp, store) = setup();
    store.init_repo("r").unwrap();

    write_file(&store, "r", "content.typ", "main\n");
    write_file(&store, "r", "content.html", "<p>cached</p>\n");
    // .ignore is a dotfile created by init_repo
    store.record("r", "initial").unwrap();

    let files = store.list_files("r").unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"content.typ"));
    assert!(!paths.contains(&"content.html"), "content.html should be skipped");
    assert!(!paths.contains(&".ignore"), "dotfiles should be skipped");
}

// ---------------------------------------------------------------------------
// fork
// ---------------------------------------------------------------------------

#[test]
fn test_fork_creates_independent_copy() {
    let (_tmp, store) = setup();
    store.init_repo("original").unwrap();

    write_file(&store, "original", "content.typ", "original content\n");
    store.record("original", "initial").unwrap();

    let fork_path = store.fork("original", "forked").unwrap();
    assert!(fork_path.exists());

    // Fork should have the same content
    let content = store.get_file_content("forked", "content.typ").unwrap();
    assert_eq!(content, b"original content\n");

    // Fork should have the same history
    let orig_log = store.log("original").unwrap();
    let fork_log = store.log("forked").unwrap();
    assert_eq!(orig_log, fork_log);
}

#[test]
fn test_fork_is_independent() {
    let (_tmp, store) = setup();
    store.init_repo("original").unwrap();

    write_file(&store, "original", "content.typ", "v1\n");
    store.record("original", "initial").unwrap();

    store.fork("original", "forked").unwrap();

    // Modify original — fork should be unaffected
    write_file(&store, "original", "content.typ", "v2\n");
    store.record("original", "update").unwrap();

    let fork_content = store.get_file_content("forked", "content.typ").unwrap();
    assert_eq!(fork_content, b"v1\n", "fork should still have original content");
}

#[test]
fn test_fork_nonexistent_source_fails() {
    let (_tmp, store) = setup();
    let result = store.fork("nonexistent", "target");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// log
// ---------------------------------------------------------------------------

#[test]
fn test_log_empty_repo() {
    let (_tmp, store) = setup();
    store.init_repo("r").unwrap();

    let log = store.log("r").unwrap();
    assert!(log.is_empty(), "fresh repo should have empty log");
}

#[test]
fn test_log_after_one_record() {
    let (_tmp, store) = setup();
    store.init_repo("r").unwrap();

    write_file(&store, "r", "content.typ", "hello\n");
    let hash = store.record("r", "first").unwrap().expect("should record");

    let log = store.log("r").unwrap();
    // pijul may include an implicit root change, so the log may have >=1 entries.
    // The hash we recorded must be present.
    assert!(log.contains(&hash), "log should contain the recorded hash");
}

#[test]
fn test_log_preserves_order() {
    let (_tmp, store) = setup();
    store.init_repo("r").unwrap();

    write_file(&store, "r", "content.typ", "v1\n");
    let h1 = store.record("r", "first").unwrap().expect("should record");

    write_file(&store, "r", "content.typ", "v2\n");
    let h2 = store.record("r", "second").unwrap().expect("should record");

    let log = store.log("r").unwrap();
    let pos1 = log.iter().position(|h| h == &h1).expect("h1 should be in log");
    let pos2 = log.iter().position(|h| h == &h2).expect("h2 should be in log");
    assert!(pos1 < pos2, "first change should appear before second in log");
}

// ---------------------------------------------------------------------------
// diff (additional edge cases)
// ---------------------------------------------------------------------------

#[test]
fn test_diff_new_file_shows_as_untracked() {
    let (_tmp, store) = setup();
    store.init_repo("r").unwrap();

    // No initial record, just put a file there
    write_file(&store, "r", "content.typ", "hello\n");

    let diff = store.diff("r").unwrap();
    // File hasn't been recorded, so it should show as untracked
    assert!(diff.untracked.contains(&"content.typ".to_string()));
}

#[test]
fn test_diff_after_revert_is_clean() {
    let (_tmp, store) = setup();
    store.init_repo("r").unwrap();

    write_file(&store, "r", "content.typ", "original\n");
    store.record("r", "initial").unwrap();

    write_file(&store, "r", "content.typ", "modified\n");
    // Should have hunks before revert
    let diff_before = store.diff("r").unwrap();
    assert!(!diff_before.hunks.is_empty());

    store.revert("r").unwrap();
    let diff_after = store.diff("r").unwrap();
    assert!(diff_after.hunks.is_empty(), "diff should be clean after revert");
}

// ---------------------------------------------------------------------------
// revert (additional)
// ---------------------------------------------------------------------------

#[test]
fn test_revert_preserves_untracked_files() {
    let (_tmp, store) = setup();
    store.init_repo("r").unwrap();

    write_file(&store, "r", "content.typ", "tracked\n");
    store.record("r", "initial").unwrap();

    // Add an untracked file and modify the tracked one
    write_file(&store, "r", "notes.txt", "untracked notes\n");
    write_file(&store, "r", "content.typ", "changed\n");

    store.revert("r").unwrap();

    // Tracked file should be reverted
    let content = store.get_file_content("r", "content.typ").unwrap();
    assert_eq!(content, b"tracked\n");

    // Untracked file should still exist
    let notes = store.get_file_content("r", "notes.txt").unwrap();
    assert_eq!(notes, b"untracked notes\n");
}

// ---------------------------------------------------------------------------
// get_file_content (additional)
// ---------------------------------------------------------------------------

#[test]
fn test_get_file_content_binary() {
    let (_tmp, store) = setup();
    store.init_repo("r").unwrap();

    let binary_data: Vec<u8> = (0..=255).collect();
    let path = store.repo_path("r").join("data.bin");
    std::fs::write(&path, &binary_data).unwrap();

    let content = store.get_file_content("r", "data.bin").unwrap();
    assert_eq!(content, binary_data);
}

// ---------------------------------------------------------------------------
// apply (additional)
// ---------------------------------------------------------------------------

#[test]
fn test_apply_nonexistent_source_fails() {
    let (_tmp, store) = setup();
    store.init_repo("target").unwrap();

    let result = store.apply("nonexistent", "target", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
    assert!(result.is_err());
}

#[test]
fn test_apply_nonexistent_target_fails() {
    let (_tmp, store) = setup();
    store.init_repo("source").unwrap();

    let result = store.apply("source", "nonexistent", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
    assert!(result.is_err());
}

#[test]
fn test_list_files_empty_repo() {
    let (_tmp, store) = setup();
    store.init_repo("test1").unwrap();

    let files = store.list_files("test1").unwrap();
    assert!(files.is_empty(), "newly initialized repo should have no tracked files");
}

#[test]
fn test_list_files_after_record() {
    let (_tmp, store) = setup();
    store.init_repo("test1").unwrap();

    write_file(&store, "test1", "content.typ", "Hello world");
    store.record("test1", "initial").unwrap();

    let files = store.list_files("test1").unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "content.typ");
    assert!(!files[0].is_dir);
}

#[test]
fn test_get_file_content() {
    let (_tmp, store) = setup();
    store.init_repo("test1").unwrap();

    write_file(&store, "test1", "content.typ", "Hello world");
    store.record("test1", "initial").unwrap();

    let content = store.get_file_content("test1", "content.typ").unwrap();
    assert_eq!(content, b"Hello world");
}

#[test]
fn test_get_file_content_not_found() {
    let (_tmp, store) = setup();
    store.init_repo("test1").unwrap();

    let result = store.get_file_content("test1", "nonexistent.typ");
    assert!(result.is_err());
}

#[test]
fn test_get_file_content_path_traversal() {
    let (_tmp, store) = setup();
    store.init_repo("test1").unwrap();

    let result = store.get_file_content("test1", "../../../etc/passwd");
    assert!(result.is_err());
}

#[test]
fn test_diff_no_changes() {
    let (_tmp, store) = setup();
    store.init_repo("test1").unwrap();

    write_file(&store, "test1", "content.typ", "Hello world\n");
    store.record("test1", "initial").unwrap();

    let diff = store.diff("test1").unwrap();
    assert!(diff.hunks.is_empty(), "no hunks expected when working copy matches pristine");
    assert!(diff.untracked.is_empty());
}

#[test]
fn test_diff_modified_file() {
    let (_tmp, store) = setup();
    store.init_repo("test1").unwrap();

    write_file(&store, "test1", "content.typ", "line one\nline two\n");
    store.record("test1", "initial").unwrap();

    // Modify the file
    write_file(&store, "test1", "content.typ", "line one\nline three\n");

    let diff = store.diff("test1").unwrap();
    assert_eq!(diff.hunks.len(), 1);
    assert_eq!(diff.hunks[0].path, "content.typ");
    assert!(diff.hunks[0].removed.contains(&"line two".to_string()));
    assert!(diff.hunks[0].added.contains(&"line three".to_string()));
}

#[test]
fn test_diff_untracked_file() {
    let (_tmp, store) = setup();
    store.init_repo("test1").unwrap();

    write_file(&store, "test1", "content.typ", "Hello\n");
    store.record("test1", "initial").unwrap();

    // Add a new untracked file
    write_file(&store, "test1", "notes.txt", "some notes\n");

    let diff = store.diff("test1").unwrap();
    assert!(diff.untracked.contains(&"notes.txt".to_string()));
}

#[test]
fn test_revert() {
    let (_tmp, store) = setup();
    store.init_repo("test1").unwrap();

    write_file(&store, "test1", "content.typ", "original content\n");
    store.record("test1", "initial").unwrap();

    // Modify the file
    write_file(&store, "test1", "content.typ", "modified content\n");

    // Verify it changed
    let content = store.get_file_content("test1", "content.typ").unwrap();
    assert_eq!(content, b"modified content\n");

    // Revert
    store.revert("test1").unwrap();

    // Content should be back to original
    let content = store.get_file_content("test1", "content.typ").unwrap();
    assert_eq!(content, b"original content\n");
}

#[test]
fn test_apply_change_between_repos() {
    let (_tmp, store) = setup();

    // Create source repo with initial content
    store.init_repo("source").unwrap();
    write_file(&store, "source", "content.typ", "original\n");
    let _hash1 = store.record("source", "initial").unwrap().expect("should record");

    // Fork to create target repo
    store.fork("source", "target").unwrap();

    // Make a change in source
    write_file(&store, "source", "content.typ", "updated\n");
    let hash2 = store.record("source", "update content").unwrap().expect("should record");

    // Apply the new change to target
    store.apply("source", "target", &hash2).unwrap();

    // Target should now have the updated content
    let content = store.get_file_content("target", "content.typ").unwrap();
    assert_eq!(content, b"updated\n");
}

#[test]
fn test_apply_already_applied() {
    let (_tmp, store) = setup();

    store.init_repo("source").unwrap();
    write_file(&store, "source", "content.typ", "hello\n");
    let hash = store.record("source", "initial").unwrap().expect("should record");

    store.fork("source", "target").unwrap();

    // Apply a change that's already in target (from the fork)
    // This should be a no-op, not an error
    store.apply("source", "target", &hash).unwrap();
}

#[test]
fn test_apply_invalid_hash() {
    let (_tmp, store) = setup();

    store.init_repo("source").unwrap();
    store.init_repo("target").unwrap();

    let result = store.apply("source", "target", "INVALIDHASH");
    assert!(result.is_err());
}

#[test]
fn test_list_files_multiple() {
    let (_tmp, store) = setup();
    store.init_repo("test1").unwrap();

    write_file(&store, "test1", "content.typ", "main content\n");
    write_file(&store, "test1", "metadata.json", "{}\n");
    store.record("test1", "initial").unwrap();

    let files = store.list_files("test1").unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"content.typ"));
    assert!(paths.contains(&"metadata.json"));
    assert_eq!(files.len(), 2);
}
