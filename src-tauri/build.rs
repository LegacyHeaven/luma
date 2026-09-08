use std::process::Command;

fn main() {
    // Embeds the git commit this binary was built from + whether the tree
    // was dirty, so the Settings page (and the debug console) can show a
    // version stamp that's actually trustworthy - "is this really the
    // build I just downloaded, or is an old process still running?" is
    // otherwise impossible for a user to answer on their own. Falls back
    // to "unknown" rather than failing the build when there's no .git
    // around (e.g. a source tarball with the git history stripped).
    let sha = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into());

    println!("cargo:rustc-env=LUMA_BUILD_SHA={sha}");

    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    println!(
        "cargo:rustc-env=LUMA_BUILD_DIRTY={}",
        if dirty { "-dirty" } else { "" }
    );

    // Rebuild if HEAD moves, so a stale sha never lingers across builds
    // done from the same checkout.
    println!("cargo:rerun-if-changed=../.git/HEAD");

    tauri_build::build()
}
