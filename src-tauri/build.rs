use std::process::Command;

fn main() {
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

    println!("cargo:rerun-if-changed=../.git/HEAD");

    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_config",
            "save_config",
            "list_themes",
            "reveal_themes_folder",
            "list_plugins",
            "get_plugin_js",
            "reveal_plugins_folder",
            "uninstall_plugin",
            "set_plugin_enabled",
            "fetch_plugin_marketplace_index",
            "install_plugin_from_url",
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
            "search_local",
            "open_app",
            "pick_app_for",
            "remove_custom_app",
            "list_locales",
            "get_locale_strings",
            "get_system_info",
            "get_debug_log",
            "clear_debug_log",
            "log_client_event",
            "reveal_log_file",
            "check_for_update",
            "apply_update",
            "fetch_marketplace_index",
            "install_theme_from_url",
            "uninstall_theme",
            "reset_to_defaults",
            "clear_browsing_data",
            "uninstall_app",
        ]),
    ))
    .expect("failed to run tauri-build");
}
