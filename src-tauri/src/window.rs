use tauri::{
    window::Color, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

/// The theme's `--color-dark-mode`. Set as the window's actual background
/// (not just the page's CSS background) so navigating from the main window
/// to settings.html - a full page load - shows this instead of a white
/// flash while the new document is loading.
const APP_BACKGROUND: Color = Color(10, 5, 16, 255);

pub const MAIN_LABEL: &str = "main";
pub const SPOTLIGHT_LABEL: &str = "spotlight";
pub const BROWSER_LABEL: &str = "browser";

/// Main window size presets (width, height) - see `WindowConfig::main_window_size`.
/// "default" here is already smaller than Luma's original 900x640, which
/// Julian felt was too big/overwhelming; "compact" is a further step down
/// for anyone who wants the window to feel closer to the spotlight itself.
pub fn main_window_dimensions(size: &str) -> (f64, f64) {
    match size {
        "compact" => (620.0, 460.0),
        "roomy" => (900.0, 640.0),
        _ => (760.0, 560.0),
    }
}

pub fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN_LABEL)
}

pub fn show_main_window(app: &AppHandle) {
    match main_window(app) {
        Some(w) => {
            let _ = w.show();
            let _ = w.unminimize();
            let _ = w.set_focus();
        }
        None => {
            let size = app
                .try_state::<crate::commands::AppState>()
                .map(|s| s.config.lock().unwrap().window.main_window_size.clone())
                .unwrap_or_else(|| "default".into());
            let (width, height) = main_window_dimensions(&size);

            if let Err(err) =
                WebviewWindowBuilder::new(app, MAIN_LABEL, WebviewUrl::App("index.html".into()))
                    .title("Luma")
                    .inner_size(width, height)
                    .min_inner_size(480.0, 360.0)
                    .center()
                    .background_color(APP_BACKGROUND)
                    .build()
            {
                crate::logging::error(app, format!("failed to recreate main window: {err}"));
            }
        }
    }
}

/// Resizes the main window (if it's currently open) to match a
/// `WindowConfig::main_window_size` preset, and re-centers it - called on
/// startup and whenever Settings saves a changed size.
pub fn apply_main_window_size(app: &AppHandle, size: &str) {
    let Some(w) = main_window(app) else {
        return;
    };
    let (width, height) = main_window_dimensions(size);
    if let Err(err) = w.set_size(tauri::Size::Logical(tauri::LogicalSize { width, height })) {
        crate::logging::error(
            app,
            format!("apply_main_window_size: set_size failed: {err}"),
        );
        return;
    }
    let _ = w.center();
}

pub fn spotlight_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(SPOTLIGHT_LABEL)
}

/// Creates the floating spotlight window if it doesn't exist yet. It starts
/// hidden - `toggle_spotlight` is what actually shows it - frameless,
/// transparent (so the theme's own rounded/blurred panel shows through),
/// always-on-top, and left out of the taskbar/dock like a launcher should be.
pub fn ensure_spotlight_window(app: &AppHandle, width: f64) -> tauri::Result<WebviewWindow> {
    if let Some(w) = spotlight_window(app) {
        return Ok(w);
    }

    let window = WebviewWindowBuilder::new(
        app,
        SPOTLIGHT_LABEL,
        WebviewUrl::App("index.html?mode=spotlight".into()),
    )
    .title("Luma")
    .inner_size(width, 128.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .shadow(false)
    .build()?;

    position_spotlight(&window, "top-center");

    Ok(window)
}

/// Places the spotlight window either dead-center or a third of the way
/// down the primary monitor (the classic Spotlight/launcher position).
pub fn position_spotlight(window: &WebviewWindow, placement: &str) {
    if placement == "center" {
        let _ = window.center();
        return;
    }

    let Ok(Some(monitor)) = window.primary_monitor() else {
        let _ = window.center();
        return;
    };

    let scale = monitor.scale_factor();
    let screen_size = monitor.size().to_logical::<f64>(scale);
    let screen_pos = monitor.position().to_logical::<f64>(scale);
    let Ok(win_size) = window.outer_size() else {
        let _ = window.center();
        return;
    };
    let win_size = win_size.to_logical::<f64>(scale);

    let x = screen_pos.x + (screen_size.width - win_size.width) / 2.0;
    let y = screen_pos.y + screen_size.height * 0.18;

    let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
}

pub fn toggle_spotlight(app: &AppHandle, width: f64, placement: &str) {
    match ensure_spotlight_window(app, width) {
        Ok(window) => {
            let visible = window.is_visible().unwrap_or(false);
            crate::logging::info(
                app,
                format!("toggle_spotlight: currently visible={visible}, toggling"),
            );
            if visible {
                let _ = window.hide();
            } else {
                position_spotlight(&window, placement);
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.emit("luma://spotlight-shown", ());
            }
        }
        Err(err) => crate::logging::error(
            app,
            format!("toggle_spotlight: failed to create spotlight window: {err}"),
        ),
    }
}

pub fn hide_spotlight(app: &AppHandle) {
    if let Some(w) = spotlight_window(app) {
        let _ = w.hide();
    }
}

/// "Built-in browser" mode: a single reusable Luma-branded webview window
/// that navigates to whatever URL you search for, instead of handing off
/// to your system browser. Uses the OS's native webview engine (Chromium
/// via WebView2 on Windows; WebKit on macOS/Linux) - see the wiki's
/// Configuration page for why that's not literally bundled Chromium everywhere.
pub fn open_in_builtin_browser(app: &AppHandle, url_str: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url_str).map_err(|e| {
        let msg = format!("open_in_builtin_browser: url::Url::parse({url_str:?}) failed: {e}");
        crate::logging::error(app, &msg);
        msg
    })?;

    if let Some(existing) = app.get_webview_window(BROWSER_LABEL) {
        crate::logging::info(
            app,
            format!("open_in_builtin_browser: reusing existing window, navigating to {parsed}"),
        );
        existing.navigate(parsed).map_err(|e| e.to_string())?;
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }

    crate::logging::info(
        app,
        format!("open_in_builtin_browser: creating new window for {parsed}"),
    );
    let toolbar_js = include_str!("../resources/builtin-browser-toolbar.js");

    WebviewWindowBuilder::new(app, BROWSER_LABEL, WebviewUrl::External(parsed))
        .title("Luma Browser")
        .inner_size(1100.0, 760.0)
        .min_inner_size(360.0, 320.0)
        .initialization_script(toolbar_js)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn close_builtin_browser(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(BROWSER_LABEL) {
        let _ = w.close();
    }
}
