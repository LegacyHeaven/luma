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
    let show_item = MenuItem::with_id(app, "show", "Open Luma", true, None::<&str>)?;
    let spotlight_item =
        MenuItem::with_id(app, "spotlight", "Toggle Spotlight", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Luma", true, None::<&str>)?;
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

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("luma.conf.json bundle.icon must provide a default window icon");

    TrayIconBuilder::with_id("luma-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Luma - press your shortcut to search")
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => window::show_main_window(app),
            "spotlight" => {
                let state = app.state::<AppState>();
                let cfg = state.config.lock().unwrap().clone();
                window::toggle_spotlight(
                    app,
                    cfg.window.spotlight_width as f64,
                    &cfg.window.spotlight_position,
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
