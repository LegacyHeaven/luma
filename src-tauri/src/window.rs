use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::{
    window::Color, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

const MAIN_THREAD_TIMEOUT: Duration = Duration::from_secs(6);

pub fn run_on_main_thread_with_timeout<T, F>(app: &AppHandle, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(f());
    })
    .map_err(|e| format!("failed to schedule work on the main thread: {e}"))?;

    rx.recv_timeout(MAIN_THREAD_TIMEOUT).map_err(|_| {
        "timed out waiting for the main thread - it may be stuck on something else; \
         try again, and if it keeps happening, restart Luma from the tray icon"
            .to_string()
    })
}

static SPOTLIGHT_GENERATION: AtomicU64 = AtomicU64::new(0);

const APP_BACKGROUND: Color = Color(10, 5, 16, 255);

const FULLY_TRANSPARENT: Color = Color(0, 0, 0, 0);

#[cfg(windows)]
fn disable_system_backdrop(app: &AppHandle, window: &WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMSBT_NONE, DWMWA_SYSTEMBACKDROP_TYPE,
    };

    let hwnd = match window.hwnd() {
        Ok(h) => h,
        Err(err) => {
            crate::logging::warn(
                app,
                format!(
                    "disable_system_backdrop: couldn't get hwnd for {}: {err}",
                    window.label()
                ),
            );
            return;
        }
    };

    let backdrop_none = DWMSBT_NONE.0;

    let result = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop_none as *const _ as *const std::ffi::c_void,
            std::mem::size_of_val(&backdrop_none) as u32,
        )
    };
    if let Err(err) = result {
        crate::logging::warn(
            app,
            format!(
                "disable_system_backdrop: DwmSetWindowAttribute failed for {}: {err}",
                window.label()
            ),
        );
    }
}

#[cfg(not(windows))]
fn disable_system_backdrop(_app: &AppHandle, _window: &WebviewWindow) {}

// Tauri's own `.transparent(true)`/`.background_color(...)` only control
// the window's compositing - the embedded WebView2 control keeps its own
// separate default background color (opaque white unless told otherwise),
// which can still paint through as a solid box around the spotlight pill
// on some Windows/WebView2 Runtime combinations even with the DWM system
// backdrop already disabled above. This reaches past Tauri into the raw
// WebView2 controller and turns that off too.
#[cfg(windows)]
fn disable_webview_background(app: &AppHandle, window: &WebviewWindow) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Controller2, COREWEBVIEW2_COLOR,
    };
    use windows::core::Interface;

    let label = window.label().to_string();
    let app_handle = app.clone();
    let result = window.with_webview(move |platform_webview| {
        let controller2 = match platform_webview.controller().cast::<ICoreWebView2Controller2>()
        {
            Ok(c) => c,
            Err(err) => {
                crate::logging::warn(
                    &app_handle,
                    format!(
                        "disable_webview_background: couldn't get ICoreWebView2Controller2 for {label}: {err}"
                    ),
                );
                return;
            }
        };

        let set_result = unsafe {
            controller2.SetDefaultBackgroundColor(COREWEBVIEW2_COLOR {
                A: 0,
                R: 0,
                G: 0,
                B: 0,
            })
        };
        if let Err(err) = set_result {
            crate::logging::warn(
                &app_handle,
                format!(
                    "disable_webview_background: SetDefaultBackgroundColor failed for {label}: {err}"
                ),
            );
        }
    });

    if let Err(err) = result {
        crate::logging::warn(
            app,
            format!("disable_webview_background: with_webview failed: {err}"),
        );
    }
}

#[cfg(not(windows))]
fn disable_webview_background(_app: &AppHandle, _window: &WebviewWindow) {}

pub const MAIN_LABEL: &str = "main";
pub const SPOTLIGHT_LABEL: &str = "spotlight";
pub const BROWSER_LABEL: &str = "browser";
pub const POSITION_PICKER_LABEL: &str = "position-picker";

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
                    .title("LUMA")
                    .inner_size(width, height)
                    .min_inner_size(480.0, 360.0)
                    .center()
                    .background_color(APP_BACKGROUND)
                    .decorations(false)
                    .build()
            {
                crate::logging::error(app, format!("failed to recreate main window: {err}"));
            }
        }
    }
}

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

const SPOTLIGHT_GLOW_MARGIN_SIDE: f64 = 50.0;
const SPOTLIGHT_WINDOW_HEIGHT: f64 = 176.0;

pub fn ensure_spotlight_window(app: &AppHandle, width: f64) -> tauri::Result<WebviewWindow> {
    if let Some(w) = spotlight_window(app) {
        return Ok(w);
    }

    let window = WebviewWindowBuilder::new(
        app,
        SPOTLIGHT_LABEL,
        WebviewUrl::App("index.html?mode=spotlight".into()),
    )
    .title("LUMA")
    .inner_size(
        width + SPOTLIGHT_GLOW_MARGIN_SIDE * 2.0,
        SPOTLIGHT_WINDOW_HEIGHT,
    )
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .background_color(FULLY_TRANSPARENT)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .shadow(false)
    .build()?;

    disable_system_backdrop(app, &window);
    disable_webview_background(app, &window);
    position_spotlight(&window, "center", None, None);

    Ok(window)
}

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

fn schedule_spotlight_hide(window: &WebviewWindow) {
    let gen = SPOTLIGHT_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = window.emit("luma://spotlight-hiding", ());
    let target = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(260));
        if SPOTLIGHT_GENERATION.load(Ordering::SeqCst) == gen {
            let _ = target.hide();
        }
    });
}

pub fn open_position_picker(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(POSITION_PICKER_LABEL) {
        let _ = w.set_focus();
        return Ok(());
    }

    let for_closure = app.clone();
    run_on_main_thread_with_timeout(app, move || {
        open_position_picker_on_main_thread(&for_closure)
    })?
}

fn open_position_picker_on_main_thread(app: &AppHandle) -> Result<(), String> {
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
    .title("LUMA")
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

    disable_system_backdrop(app, &window);
    disable_webview_background(app, &window);
    let _ = window.set_focus();
    Ok(())
}

pub fn close_position_picker(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(POSITION_PICKER_LABEL).is_none() {
        return Ok(());
    }
    let for_closure = app.clone();
    run_on_main_thread_with_timeout(app, move || {
        if let Some(w) = for_closure.get_webview_window(POSITION_PICKER_LABEL) {
            let _ = w.close();
        }
    })
}

pub fn open_in_builtin_browser(app: &AppHandle, url_str: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url_str).map_err(|e| {
        let msg = format!("open_in_builtin_browser: url::Url::parse({url_str:?}) failed: {e}");
        crate::logging::error(app, &msg);
        msg
    })?;

    let for_closure = app.clone();
    run_on_main_thread_with_timeout(app, move || {
        open_in_builtin_browser_on_main_thread(&for_closure, parsed)
    })?
}

fn open_in_builtin_browser_on_main_thread(app: &AppHandle, parsed: url::Url) -> Result<(), String> {
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
    let toolbar_js = include_str!("../resources/builtin-browser-toolbar.js")
        .replace("__THEME_VARS_JSON__", &theme_vars_json(app));

    WebviewWindowBuilder::new(app, BROWSER_LABEL, WebviewUrl::External(parsed))
        .title("LUMA Browser")
        .inner_size(1100.0, 760.0)
        .min_inner_size(360.0, 320.0)
        .initialization_script(&toolbar_js)
        .decorations(false)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn theme_vars_json(app: &AppHandle) -> String {
    const WANTED: &[(&str, &str, &str)] = &[
        ("luma-bg", "box-first-color", "#180d29"),
        ("luma-border", "box-border-color", "#3a1f5c"),
        ("luma-accent", "color-light-purple", "#cf59e6"),
        ("luma-text", "color-white", "#fff"),
        ("luma-muted", "color-gray", "#c4c4c4"),
        (
            "luma-font",
            "font-family",
            "'Fira Code', 'JetBrains Mono', 'Cascadia Code', monospace",
        ),
    ];

    let theme_id = app
        .try_state::<crate::commands::AppState>()
        .map(|s| s.config.lock().unwrap().appearance.theme.clone())
        .unwrap_or_else(|| "luma-default".into());
    let css = crate::themes::css_for(app, &theme_id).unwrap_or_default();

    let mut map = serde_json::Map::new();
    for (toolbar_name, theme_var, fallback) in WANTED {
        let value = find_css_var(&css, theme_var).unwrap_or_else(|| (*fallback).to_string());
        map.insert(
            (*toolbar_name).to_string(),
            serde_json::Value::String(value),
        );
    }
    serde_json::Value::Object(map).to_string()
}

fn find_css_var(css: &str, name: &str) -> Option<String> {
    let prefix = format!("--{name}:");
    for line in css.lines() {
        if let Some(rest) = line.trim().strip_prefix(&prefix) {
            let value = rest.split(';').next().unwrap_or(rest).trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub fn close_builtin_browser(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(BROWSER_LABEL).is_none() {
        return Ok(());
    }
    let for_closure = app.clone();
    run_on_main_thread_with_timeout(app, move || {
        if let Some(w) = for_closure.get_webview_window(BROWSER_LABEL) {
            let _ = w.close();
        }
    })
}
