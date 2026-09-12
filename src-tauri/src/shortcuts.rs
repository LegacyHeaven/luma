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
// this as a fallback for Escape-to-cancel. Its own in-page `keydown`
// listener only fires while the picker's webview actually holds OS
// keyboard focus - and on Windows that turned out not to be reliable for
// every open of the picker (a `WebviewWindow::set_focus()` called right
// after creating an always-on-top, decorationless, transparent window can
// silently lose the race to the window that was focused a moment earlier,
// especially on the second or later time the picker is opened in the same
// session - confirmed live: clicking to place a spot always worked since
// that's routed by screen position rather than focus, but a plain Escape
// keypress sometimes reached the *previous* focused window instead of the
// picker and did nothing, leaving the exact "stuck overlay with no way
// out" trap this picker already had one bug for).
//
// A global shortcut sidesteps the whole problem: `global-hotkey` registers
// it directly with the OS (RegisterHotKey on Windows), so it fires no
// matter which window currently has focus. It's registered only while the
// picker is open and unregistered the moment it closes (by any means -
// placing a spot, Escape, or losing focus), so it never shadows a plain
// Escape press anywhere else in the app or on the desktop the rest of the
// time.
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
