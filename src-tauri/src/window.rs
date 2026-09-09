use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{
    window::Color, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

/// Bumped on every show/hide of the spotlight so a delayed "actually hide
/// now" (see toggle_spotlight/hide_spotlight) can tell whether it's still
/// the most recent request before it fires - otherwise a fast
/// hide-then-show within the fade-out window would hide a spotlight the
/// user just reopened. Process-wide is fine: there's only ever one
/// spotlight window.
static SPOTLIGHT_GENERATION: AtomicU64 = AtomicU64::new(0);

/// The theme's `--color-dark-mode`. Set as the window's actual background
/// (not just the page's CSS background) so navigating from the main window
/// to settings.html - a full page load - shows this instead of a white
/// flash while the new document is loading.
const APP_BACKGROUND: Color = Color(10, 5, 16, 255);

/// Fully transparent (alpha 0). `transparent: true` on the WebviewWindow
/// alone isn't enough on Windows - WebView2 still paints its own opaque
/// default background wherever the page doesn't, which shows up as a hard
/// rectangular box around the spotlight pill instead of the pill floating
/// free on the wallpaper. Explicitly setting the webview's own background
/// to this is what actually gets rid of it (Windows-only in practice;
/// harmless to set everywhere).
const FULLY_TRANSPARENT: Color = Color(0, 0, 0, 0);

pub const MAIN_LABEL: &str = "main";
pub const SPOTLIGHT_LABEL: &str = "spotlight";
pub const BROWSER_LABEL: &str = "browser";
pub const POSITION_PICKER_LABEL: &str = "position-picker";

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
    .background_color(FULLY_TRANSPARENT)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .shadow(false)
    .build()?;

    position_spotlight(&window, "center", None, None);

    Ok(window)
}

/// Places the spotlight window - "center" (the default, dead-center on the
/// primary monitor) or "custom" (centered on a point the user picked with
/// the Settings "Pick position" click-to-choose overlay, stored as a
/// fraction of the primary monitor's size so it survives a resolution
/// change reasonably). Falls back to center whenever a custom position
/// isn't actually available, so a bad/missing value can never strand the
/// spotlight off-screen.
pub fn position_spotlight(
    window: &WebviewWindow,
    placement: &str,
    x_frac: Option<f64>,
    y_frac: Option<f64>,
) {
    if placement != "custom" {
        let _ = window.center();
        return;
    }

    let (Some(x_frac), Some(y_frac)) = (x_frac, y_frac) else {
        let _ = window.center();
        return;
    };

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

    // The clicked point becomes the *center* of the spotlight, not its
    // top-left corner - that's what "spawn from the middle there" means.
    let x = screen_pos.x + x_frac * screen_size.width - win_size.width / 2.0;
    let y = screen_pos.y + y_frac * screen_size.height - win_size.height / 2.0;

    let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
}

pub fn toggle_spotlight(
    app: &AppHandle,
    width: f64,
    placement: &str,
    x_frac: Option<f64>,
    y_frac: Option<f64>,
) {
    match ensure_spotlight_window(app, width) {
        Ok(window) => {
            let visible = window.is_visible().unwrap_or(false);
            crate::logging::info(
                app,
                format!("toggle_spotlight: currently visible={visible}, toggling"),
            );
            if visible {
                schedule_spotlight_hide(&window);
            } else {
                SPOTLIGHT_GENERATION.fetch_add(1, Ordering::SeqCst);
                position_spotlight(&window, placement, x_frac, y_frac);
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
        schedule_spotlight_hide(&w);
    }
}

/// Lets the frontend play its fade-out animation before the native window
/// actually disappears (see main.js's "luma://spotlight-hiding" listener),
/// with a fixed grace period as a floor - so it hides promptly even with
/// animations disabled, or if the page never got a chance to react. Guards
/// against a fast hide-then-show race with SPOTLIGHT_GENERATION: if
/// anything else (another hide, or a show) happened after this one was
/// scheduled, this call is a no-op instead of hiding a window the user
/// just reopened.
fn schedule_spotlight_hide(window: &WebviewWindow) {
    let gen = SPOTLIGHT_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = window.emit("luma://spotlight-hiding", ());
    let target = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(220));
        if SPOTLIGHT_GENERATION.load(Ordering::SeqCst) == gen {
            let _ = target.hide();
        }
    });
}

/// Fullscreen, click-through-free overlay on the primary monitor, used by
/// Settings' "Pick position" button. Reports the click back as a fraction
/// of the monitor's size (see position_spotlight) and closes itself either
/// way - clicking picks a spot, Escape cancels without changing anything.
pub fn open_position_picker(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(POSITION_PICKER_LABEL) {
        let _ = w.set_focus();
        return Ok(());
    }

    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no primary monitor found".to_string())?;
    let scale = monitor.scale_factor();
    let size = monitor.size().to_logical::<f64>(scale);
    let pos = monitor.position().to_logical::<f64>(scale);

    let window = WebviewWindowBuilder::new(
        app,
        POSITION_PICKER_LABEL,
        WebviewUrl::App("position-picker.html".into()),
    )
    .title("Luma")
    .inner_size(size.width, size.height)
    .position(pos.x, pos.y)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .background_color(FULLY_TRANSPARENT)
    .always_on_top(true)
    .skip_taskbar(true)
    .build()
    .map_err(|e| e.to_string())?;

    let _ = window.set_focus();
    Ok(())
}

pub fn close_position_picker(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(POSITION_PICKER_LABEL) {
        let _ = w.close();
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
