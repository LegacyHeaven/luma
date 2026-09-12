use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut};

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

// The position-picker overlay (see window::open_position_picker) relies on
// this as a *secondary* fallback for Escape-to-cancel - the real fix for
// the bug that motivated it (see the long comment in window.rs next to
// where the picker window is built) is re-asserting window focus shortly
// after the window opens. This exists on top of that because a global
// hotkey fires no matter which window currently has focus, at least when
// registering it succeeds - and it doesn't always: confirmed live, some
// other already-running app can already hold a bare Escape as *its own*
// global hotkey, which makes this registration fail outright (global
// hotkeys are exclusive system-wide, one owner at a time, so this can
// never be assumed to succeed). When that happens this quietly does
// nothing and the picker falls back to the focus fix plus its own in-page
// keydown handler, same as if this didn't exist at all. It's registered
// only while the picker is open and unregistered the moment it closes (by
// any means - placing a spot, Escape, or losing focus), so on the machines
// where it *can* register, it never shadows a plain Escape press anywhere
// else in the app or on the desktop the rest of the time.
pub fn position_picker_cancel_shortcut() -> Shortcut {
    Shortcut::new(None, Code::Escape)
}

pub fn register_position_picker_escape(app: &AppHandle) -> Result<(), String> {
    app.global_shortcut()
        .register(position_picker_cancel_shortcut())
        .map_err(|e| e.to_string())
}

pub fn unregister_position_picker_escape(app: &AppHandle) -> Result<(), String> {
    app.global_shortcut()
        .unregister(position_picker_cancel_shortcut())
        .map_err(|e| e.to_string())
}
