//! Download and cache remote avatars (e.g. Bluesky CDN URLs) to local disk
//! so the browser always loads them from our own `/api/avatars/{did}` endpoint.
//!
//! Returns the rewritten local URL, or `None` if the download failed or the
//! input was already local.

use std::path::Path;

const VALID_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp"];
const MAX_BYTES: usize = 4 * 1024 * 1024; // 4 MB sanity cap

fn safe_did(did: &str) -> String {
    did.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.' || *c == ':')
        .collect()
}

/// Pick an extension from the Content-Type header.
fn ext_from_ct(ct: &str) -> &'static str {
    match ct {
        c if c.contains("png") => "png",
        c if c.contains("webp") => "webp",
        _ => "jpg",
    }
}

/// Download `url` and write it under `{data_dir}/avatars/{safe_did}.{ext}`.
///
/// If `url` already points at this server (`/api/avatars/...`), returns it
/// unchanged. On any failure (network, wrong content-type, oversized) returns
/// `None` so callers can fall back to the remote URL rather than crash.
pub async fn cache_remote_avatar(
    data_dir: &Path,
    did: &str,
    url: &str,
) -> Option<String> {
    if url.starts_with("/api/avatars/") {
        return Some(url.to_string());
    }

    let resp = reqwest::get(url).await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    let ext = ext_from_ct(&ct);

    let bytes = resp.bytes().await.ok()?;
    if bytes.len() > MAX_BYTES {
        return None;
    }

    let safe = safe_did(did);
    let avatars_dir = data_dir.join("avatars");
    tokio::fs::create_dir_all(&avatars_dir).await.ok()?;

    // Remove any stale copy under a different extension so `get_avatar` can't
    // serve outdated bytes from a different format.
    for stale in VALID_EXTS {
        if *stale == ext {
            continue;
        }
        let _ = tokio::fs::remove_file(avatars_dir.join(format!("{safe}.{stale}"))).await;
    }

    tokio::fs::write(avatars_dir.join(format!("{safe}.{ext}")), &bytes)
        .await
        .ok()?;

    Some(format!("/api/avatars/{safe}"))
}
