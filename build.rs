//! Build-time metadata:
//! 1. Git info for `naysay --version` (falls back to "unknown" outside a repo).
//! 2. Windows executable icon + version resource (assets/naysay.ico).

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
    println!("cargo:rerun-if-changed=assets/naysay.ico");

    println!("cargo:rustc-env=NAYSAY_GIT_HASH={hash}");
    println!("cargo:rustc-env=NAYSAY_GIT_TAG={tag}");

    // Windows: embed the icon + version info into the .exe. Skipped on
    // other platforms — the icon is a Windows Explorer concern.
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        if !std::path::Path::new("assets/naysay.ico").exists() {
            println!("cargo:warning=assets/naysay.ico missing — exe will have no icon");
            return;
        }
        match winresource::WindowsResource::new()
            .set_icon("assets/naysay.ico")
            .set(
                "FileDescription",
                &format!("naysay v{}", env!("CARGO_PKG_VERSION")),
            )
            .set("ProductName", "naysay")
            .compile()
        {
            Ok(()) => {}
            Err(e) => {
                // A missing/broken resource toolchain must not kill the build —
                // the icon is cosmetic.
                println!("cargo:warning=icon embedding skipped: {e}");
            }
        }
    }
}
