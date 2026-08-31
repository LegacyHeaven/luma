use tauri::AppHandle;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

/// Parses a human-typed shortcut string (e.g. "Alt+Space", "CommandOrControl+Shift+Space")
/// and registers it as Luma's spotlight-toggle hotkey.
pub fn register(app: &AppHandle, shortcut_str: &str) -> Result<(), String> {
    let shortcut: tauri_plugin_global_shortcut::Shortcut = shortcut_str
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

/// Swaps the active hotkey for a new one - used when the user changes it
/// from Settings, so the change applies without restarting Luma.
pub fn reregister(app: &AppHandle, shortcut_str: &str) -> Result<(), String> {
    unregister_all(app)?;
    register(app, shortcut_str)
}
