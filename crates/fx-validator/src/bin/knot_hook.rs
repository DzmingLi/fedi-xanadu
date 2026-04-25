//! `fx-validator-knot-hook` — pre-apply hook binary for pijul knots that host
//! NightBoat content.
//!
//! # Wiring
//!
//! In a repo's `.pijul/config.toml`:
//!
//! ```toml
//! [[hooks.pre_apply]]
//! command = "fx-validator-knot-hook"
//! args = []
//! ```
//!
//! pijul sets `PIJUL_CHANGE_HASH`, `PIJUL_CHANNEL`, optionally `PIJUL_PUSHER`
//! in the env and runs the binary with cwd = repo working-copy. If the hook
//! exits non-zero, pijul aborts the apply and (for remote pushes) rejects the
//! push; stderr is forwarded to the client.
//!
//! # What this hook does
//!
//! 1. Scans the working copy (cwd) for `.md` and `main.typ` files.
//! 2. Extracts article metadata from each (MD frontmatter / Typst
//!    `#metadata() <nightboat-translation>`).
//! 3. Runs cross-file validation (no translation chains, no duplicate langs,
//!    missing `lang`, etc.).
//! 4. Prints a human-readable error report to stderr and exits 1 on any
//!    violation. Clean scans exit 0.
//!
//! The hook is stateless and has no NightBoat DB access — it's purely the
//! filesystem + the validator rules. NightBoat's indexer still runs after
//! the apply to reconcile the DB; this hook is an optional front gate that
//! rejects bad content at push time.
//!
//! # Operators who don't want validation
//!
//! Simply don't add the hook. The knot is pijul-native and agnostic —
//! NightBoat rules only apply when this binary is wired in.

use std::path::Path;
use std::process::ExitCode;

use fx_validator::{scan, validate_files, scan::ScanErrorKind};

fn main() -> ExitCode {
    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let (metas, scan_errors) = scan::scan_dir(&cwd);

    // Surface per-file scan errors first — these are already fatal for the
    // individual file even if cross-file validation would have been clean.
    let mut had_errors = false;
    for err in &scan_errors {
        match &err.error {
            ScanErrorKind::Io(msg) => {
                eprintln!(
                    "fx-validator: IO error reading {}: {msg}",
                    err.path.display()
                );
                had_errors = true;
            }
            ScanErrorKind::Extract(verr) => {
                eprintln!("fx-validator: {}", verr);
                had_errors = true;
            }
        }
    }

    // Cross-file rules (translation chains, duplicate langs, etc.).
    let (_articles, report) = validate_files(metas);
    for err in &report.errors {
        eprintln!("fx-validator: {}", err);
        had_errors = true;
    }

    if had_errors {
        eprintln!();
        eprintln!(
            "Push rejected: {} file error(s), {} cross-file error(s).",
            scan_errors.len(),
            report.errors.len()
        );
        eprintln!(
            "See https://nightbo.at/guide/translation for the file metadata contract."
        );
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}
