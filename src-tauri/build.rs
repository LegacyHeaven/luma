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

    // Registers every one of Luma's own commands as ones Tauri's ACL
    // system knows how to permission (autogenerates an `allow-<command>`
    // permission for each, kebab-cased - see capabilities/default.json and
    // capabilities/builtin-browser-remote.json, which grant them).
    //
    // IMPORTANT, hard-won the expensive way (2.1.1 round 2 shipped this
    // broken): defining an app manifest AT ALL - even for just one or two
    // commands - flips a single *global* switch inside Tauri
    // (`RuntimeAuthority::has_app_manifest()`, set once from whether the
    // ACL map has an app entry at all, not per-window or per-command) that
    // makes it start enforcing ACL on *every* app-defined command, from
    // *every* window, local content included. Before this file defined a
    // manifest at all (`tauri_build::build()`, no `AppManifest`), local
    // content was implicitly trusted for every app command with zero ACL
    // needed - which is why the built-in browser toolbar's two buttons
    // needing a manifest+capability at all was surprising in the first
    // place. The instant a manifest exists for *any* command, that
    // blanket local trust is gone for the *whole app*, and every command
    // needs an explicit `allow-*` permission in some capability that
    // covers its window, or every window's IPC calls start failing ACL
    // and the UI breaks outright (get_config, get_engines, etc. all
    // silently rejected - this is exactly what happened). So: every
    // command in main.rs's `generate_handler!` list has to be listed here
    // too, and granted in capabilities/default.json - not just the ones
    // that strictly need a *new* grant.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_config",
            "save_config",
            "list_themes",
            "reveal_themes_folder",
            "get_engines",
            "get_theme_css",
            "open_result",
            "open_in_system_browser",
            "close_builtin_browser",
            "toggle_spotlight",
            "hide_spotlight",
            "show_main_window",
            "open_position_picker",
            "report_spotlight_position",
            "cancel_position_pick",
            "reset_spotlight_position",
            "add_custom_engine",
            "remove_custom_engine",
            "list_all_builtin_engines",
            "set_builtin_engine_enabled",
            "search_mypc",
            "open_app",
            "pick_app_for",
            "remove_custom_app",
            "get_system_info",
            "get_debug_log",
            "clear_debug_log",
            "log_client_event",
            "check_for_update",
            "apply_update",
        ]),
    ))
    .expect("failed to run tauri-build");
}
