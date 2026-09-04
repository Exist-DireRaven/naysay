//! Capture git metadata at build time so `naysay --version` can show it.
//! Falls back to "unknown" when git is unavailable (e.g. release tarball, CI cache).

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn main() {
    let hash = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let tag = git(&["describe", "--tags", "--always"]).unwrap_or_else(|| "unknown".into());

    // Only rerun if HEAD changes.
    println!("cargo:rerun-if-changed=.git/HEAD");

    println!("cargo:rustc-env=NAYSAY_GIT_HASH={hash}");
    println!("cargo:rustc-env=NAYSAY_GIT_TAG={tag}");
}
