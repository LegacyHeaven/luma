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

    // Registers close_builtin_browser/open_in_system_browser as commands
    // Tauri's ACL system knows how to permission (autogenerates
    // `allow-close-builtin-browser` / `allow-open-in-system-browser`, see
    // capabilities/builtin-browser-remote.json). Without an app manifest
    // like this, Tauri has no manifest entry to grant those commands
    // *against at all* - it doesn't matter what a capability's JSON says,
    // there's nothing for it to reference - and a plain `tauri_build::build()`
    // never creates one. That's the actual reason the built-in browser's
    // toolbar buttons ("Open in system browser", the "x" close button) did
    // nothing: every custom command is already open to *local* Luma content
    // with no ACL needed, but Tauri always enforces ACL for commands called
    // from *remote* content - which is exactly what the toolbar is, since
    // it's injected into whatever external site the user searched to - and
    // there was no manifest entry for it to possibly be allowed under.
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new()
                .commands(&["close_builtin_browser", "open_in_system_browser"]),
        ),
    )
    .expect("failed to run tauri-build");
}
