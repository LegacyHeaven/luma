use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub fn register(app: &AppHandle, shortcut_str: &str) -> Result<(), String> {
    let shortcut: Shortcut = shortcut_str
        .parse()
        .map_err(|e| format!("invalid shortcut '{shortcut_str}': {e}"))?;

    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| e.to_string())
}

pub fn unregister_all(app: &AppHandle) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())
}

pub fn reregister(app: &AppHandle, shortcut_str: &str) -> Result<(), String> {
    unregister_all(app)?;
    register(app, shortcut_str)
}
