//! Shared utility functions for ID generation, hashing, and string manipulation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a time-sortable ID using microsecond timestamp in base32.
///
/// Format: 13 chars of base32-encoded microseconds since epoch (AT Protocol TID).
/// Uses an atomic counter to guarantee uniqueness even within the same microsecond.
pub fn tid() -> String {
    static LAST_TID: AtomicU64 = AtomicU64::new(0);

    let micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_micros() as u64;

    let val = loop {
        let prev = LAST_TID.load(Ordering::Relaxed);
        let next = if micros > prev { micros } else { prev + 1 };
        if LAST_TID
            .compare_exchange(prev, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            break next;
        }
    };

    const CHARS: &[u8] = b"234567abcdefghijklmnopqrstuvwxyz";
    let mut out = [0u8; 13];
    let mut v = val;
    for byte in out.iter_mut().rev() {
        *byte = CHARS[(v & 0x1f) as usize];
        v >>= 5;
    }
    // Safety: all bytes are ASCII from the CHARS array
    String::from_utf8(out.to_vec()).expect("TID is always valid ASCII")
}

/// Blake3 content hash as hex string.
pub fn content_hash(data: &str) -> String {
    blake3::hash(data.as_bytes()).to_hex().to_string()
}

/// Generate a cryptographically random 64-char hex session token.
pub fn gen_session_token() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let bytes: [u8; 32] = rng.random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Convert an AT URI to a filesystem-safe node ID.
pub fn uri_to_node_id(uri: &str) -> String {
    uri.replace('/', "_").replace(':', "_")
}

/// Parse `repo_uri = at://{did}/{lex}/{rkey}` into `(did, rkey)` for use as
/// the `(owner, name)` segment pair in a pijangle-knot URL
/// (`<knot>/{did}/{rkey}/`). Returns `None` for non-`at://` URIs — notably
/// `server://qa/...` (server-only Q&A, no knot repo) and the synthetic
/// `nightboat://article/...` form.
pub fn parse_repo_uri(repo_uri: &str) -> Option<(String, String)> {
    let rest = repo_uri.strip_prefix("at://")?;
    let mut parts = rest.splitn(3, '/');
    let did = parts.next()?;
    let _collection = parts.next()?;
    let rkey = parts.next()?;
    if did.is_empty() || rkey.is_empty() {
        return None;
    }
    Some((did.to_string(), rkey.to_string()))
}

/// Current UTC time as RFC 3339 string.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tid_length_is_13() {
        let t = tid();
        assert_eq!(t.len(), 13, "TID should be 13 characters, got: {t}");
    }

    #[test]
    fn tid_is_ascii_base32() {
        let t = tid();
        let valid = b"234567abcdefghijklmnopqrstuvwxyz";
        for ch in t.bytes() {
            assert!(valid.contains(&ch), "unexpected char '{}' in TID", ch as char);
        }
    }

    #[test]
    fn tid_is_sortable() {
        let t1 = tid();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let t2 = tid();
        assert!(t2 > t1, "TIDs should be time-sortable: {t1} vs {t2}");
    }

    #[test]
    fn tid_uniqueness() {
        let tids: Vec<String> = (0..1000).map(|_| tid()).collect();
        let set: std::collections::HashSet<&String> = tids.iter().collect();
        assert_eq!(set.len(), tids.len(), "TIDs should be unique even in a tight loop");
    }

    #[test]
    fn tid_monotonically_increasing() {
        let tids: Vec<String> = (0..100).map(|_| tid()).collect();
        for pair in tids.windows(2) {
            assert!(
                pair[1] > pair[0],
                "TIDs should be strictly increasing: {} vs {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn content_hash_deterministic() {
        assert_eq!(content_hash("hello"), content_hash("hello"));
    }

    #[test]
    fn content_hash_differs_for_different_input() {
        assert_ne!(content_hash("hello"), content_hash("world"));
    }

    #[test]
    fn content_hash_is_hex_string() {
        let h = content_hash("test");
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()), "hash should be hex: {h}");
        assert_eq!(h.len(), 64, "blake3 hex hash should be 64 chars");
    }

    #[test]
    fn session_token_length() {
        assert_eq!(gen_session_token().len(), 64);
    }

    #[test]
    fn session_token_is_hex() {
        assert!(gen_session_token().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn session_token_uniqueness() {
        assert_ne!(gen_session_token(), gen_session_token());
    }

    #[test]
    fn uri_to_node_id_replaces_slashes_and_colons() {
        let node_id = uri_to_node_id("at://did:plc:abc/app.bsky.feed.post/123");
        assert!(!node_id.contains('/'));
        assert!(!node_id.contains(':'));
    }

    #[test]
    fn parse_repo_uri_extracts_did_and_rkey() {
        let (did, rkey) = parse_repo_uri("at://did:plc:abc/at.nightbo.work/quantum-intro").unwrap();
        assert_eq!(did, "did:plc:abc");
        assert_eq!(rkey, "quantum-intro");
    }

    #[test]
    fn parse_repo_uri_rejects_non_at_scheme() {
        assert!(parse_repo_uri("server://qa/abc").is_none());
        assert!(parse_repo_uri("nightboat://article/at://foo/lex/bar/sub.md").is_none());
    }

    #[test]
    fn parse_repo_uri_rejects_truncated_paths() {
        assert!(parse_repo_uri("at://did:plc:abc").is_none());
        assert!(parse_repo_uri("at://did:plc:abc/at.nightbo.work").is_none());
        assert!(parse_repo_uri("at://did:plc:abc/at.nightbo.work/").is_none());
    }

    #[test]
    fn now_rfc3339_parses() {
        let ts = now_rfc3339();
        chrono::DateTime::parse_from_rfc3339(&ts).expect("should be valid RFC3339");
    }
}
