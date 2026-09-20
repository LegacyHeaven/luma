use crate::{commands::AppState, locales, window};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, Wry,
};

pub struct TrayMenuItems {
    pub show: MenuItem<Wry>,
    pub spotlight: MenuItem<Wry>,
    pub settings: MenuItem<Wry>,
    pub quit: MenuItem<Wry>,
}

pub fn retext(app: &AppHandle, locale_id: &str) {
    let strings = locales::strings_for(app, locale_id);
    let text = |key: &str, fallback: &str| {
        strings
            .get(key)
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    };

    let items = app.state::<TrayMenuItems>();
    let _ = items.show.set_text(text("tray.open", "Open LUMA"));
    let _ = items
        .spotlight
        .set_text(text("tray.toggle_spotlight", "Toggle Spotlight"));
    let _ = items.settings.set_text(text("tray.settings", "Settings…"));
    let _ = items.quit.set_text(text("tray.quit", "Quit LUMA"));

    if let Some(tray) = app.tray_by_id("luma-tray") {
        let _ = tray.set_tooltip(Some(text(
            "tray.tooltip",
            "LUMA - press your shortcut to search",
        )));
    }
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Open LUMA", true, None::<&str>)?;
    let spotlight_item =
        MenuItem::with_id(app, "spotlight", "Toggle Spotlight", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit LUMA", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    app.manage(TrayMenuItems {
        show: show_item.clone(),
        spotlight: spotlight_item.clone(),
        settings: settings_item.clone(),
        quit: quit_item.clone(),
    });

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
                    cfg.general.show_spotlight_branding,
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

    let locale = app
        .state::<AppState>()
        .config
        .lock()
        .unwrap()
        .general
        .locale
        .clone();
    retext(app, &locale);

    Ok(())
}
