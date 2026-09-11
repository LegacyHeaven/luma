use crate::{commands::AppState, window};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

/// Builds the system tray icon + menu. Luma keeps running here even when
/// every window is hidden, which is what lets the global spotlight shortcut
/// keep working - closing the main window hides it rather than quitting.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Open LUMA", true, None::<&str>)?;
    let spotlight_item =
        MenuItem::with_id(app, "spotlight", "Toggle Spotlight", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit LUMA", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &spotlight_item,
            &separator,
            &settings_item,
            &separator,
            &quit_item,
        ],
    )?;

    // Deliberately not `app.default_window_icon()` here: on Windows that
    // decodes only the *first* frame stored inside icons/icon.ico, which is
    // the 16x16 entry - Windows then has to stretch that tiny bitmap up for
    // any DPI above 100%, which is exactly why the tray/taskbar icon looked
    // "very low res" even once it was showing the right artwork. Loading a
    // real 128x128 PNG directly gives the OS a source big enough to scale
    // down cleanly at any DPI instead.
    let icon = tauri::include_image!("icons/128x128.png");

    TrayIconBuilder::with_id("luma-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("LUMA - press your shortcut to search")
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => window::show_main_window(app),
            "spotlight" => {
                let state = app.state::<AppState>();
                let cfg = state.config.lock().unwrap().clone();
                window::toggle_spotlight(
                    app,
                    cfg.window.spotlight_width as f64,
                    &cfg.window.spotlight_position,
                    cfg.window.spotlight_custom_x,
                    cfg.window.spotlight_custom_y,
                );
            }
            "settings" => {
                window::show_main_window(app);
                if let Some(w) = window::main_window(app) {
                    let _ = w.eval("window.location.href = 'settings.html';");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}
