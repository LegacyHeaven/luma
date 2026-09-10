use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::{
    window::Color, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

/// How long a main-thread-affine window operation gets before we give up on
/// it - see `run_on_main_thread_with_timeout` below. Long enough that a
/// slow-but-working machine never trips it under normal use, short enough
/// that Julian isn't staring at a frozen app for more than a few seconds
/// before Luma admits something's wrong.
const MAIN_THREAD_TIMEOUT: Duration = Duration::from_secs(6);

/// Window creation/navigation on Windows (Win32 + WebView2/COM) and on
/// macOS (AppKit) has to happen on the main thread. Every `#[tauri::command]`
/// runs on a tokio worker thread, so Tauri has to hop over to the main
/// thread internally to actually build or navigate a window - and if that
/// hop ever wedges (a COM call that never returns, a re-entrant deadlock,
/// anything), the *whole* main thread's message pump goes down with it.
/// That's the "in-app browser freezes the whole app, and even the debug
/// log stops responding" bug Julian reported: the debug console's own
/// `get_debug_log` calls are themselves commands running on that same
/// worker pool, so once enough of them are piled up waiting on a wedged
/// main-thread hop, nothing IPC-based works any more, in any window.
///
/// This is the general failsafe Julian asked for: instead of waiting on
/// that hop forever, wait at most `timeout`, and report a clean error the
/// caller can show ("failed to open - try again") instead of taking the
/// rest of the app down with it. Window-creation code should always go
/// through this rather than calling `WebviewWindowBuilder::build()` (or
/// `.navigate()`/`.close()` on a window that might not exist yet) directly
/// from a command handler.
pub fn run_on_main_thread_with_timeout<T, F>(app: &AppHandle, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        // If we already timed out and returned, the receiver is gone -
        // send() failing here is expected in that case, not a new bug.
        let _ = tx.send(f());
    })
    .map_err(|e| format!("failed to schedule work on the main thread: {e}"))?;

    rx.recv_timeout(MAIN_THREAD_TIMEOUT).map_err(|_| {
        "timed out waiting for the main thread - it may be stuck on something else; \
         try again, and if it keeps happening, restart Luma from the tray icon"
            .to_string()
    })
}

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

/// Windows 11 only: turns off DWM's automatic Mica/system-backdrop material
/// on an undecorated, per-pixel-transparent window.
///
/// `transparent(true)` + `background_color(FULLY_TRANSPARENT)` above are
/// the whole story on Windows 10 and on macOS/Linux - but on Windows 11,
/// DWM composites its own default backdrop (Mica, as of 22H2) behind any
/// undecorated ("popup"-style) window unless told not to, *on top of* the
/// window's own transparency. The window is still genuinely per-pixel
/// transparent underneath - this is a separate compositor-level tint DWM
/// adds for windows it thinks want a "frosted glass" look - and it's what
/// was actually showing up as a solid dark box around the spotlight pill
/// even though every CSS and Tauri-level transparency setting was already
/// correct (confirmed against a live screenshot from Windows 11, and by
/// reading tao 0.35.3's own window-creation code: it calls the legacy
/// `DwmEnableBlurBehindWindow` Vista/7-era API for `transparent: true`, but
/// never the modern `DWMWA_SYSTEMBACKDROP_TYPE` attribute Windows 11 needs).
/// `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_NONE)` is that
/// missing call - safe to make on every Windows version (older DWM just
/// ignores an attribute it doesn't recognize), so this isn't OS-version-gated.
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
    // Safety: `hwnd` comes straight from the just-built window (still valid
    // and owned by this process), and `backdrop_none` is a plain i32 whose
    // address and size we pass through exactly as DwmSetWindowAttribute
    // requires - this mirrors the C usage in Microsoft's own DWM docs.
    let result = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop_none as *const _ as *const std::ffi::c_void,
            std::mem::size_of_val(&backdrop_none) as u32,
        )
    };
    if let Err(err) = result {
        // Not fatal - worst case the window looks the way it did before
        // this fix (a visible backdrop box), not broken in some new way.
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
                    .title("LUMA")
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
    .title("LUMA")
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

    disable_system_backdrop(app, &window);
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
///
/// Called from the `open_position_picker` command, which is `async fn` -
/// see that command's doc comment for why that matters: `WebviewWindowBuilder::build()`
/// is documented to deadlock Windows/WebView2 when called from a
/// *synchronous* command (https://github.com/tauri-apps/wry/issues/583),
/// which is exactly the bug Julian hit ("the pick position fails the same
/// way as the in-app browser"). Being async gets this off the main thread
/// so the `run_on_main_thread_with_timeout` hop below can actually
/// complete instead of deadlocking against itself.
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
    // Re-check now that we're actually on the main thread - two rapid
    // clicks of "Pick position" could otherwise both pass the check above
    // and race to create the same-labeled window twice.
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

/// "Built-in browser" mode: a single reusable Luma-branded webview window
/// that navigates to whatever URL you search for, instead of handing off
/// to your system browser. Uses the OS's native webview engine (Chromium
/// via WebView2 on Windows; WebKit on macOS/Linux) - see the wiki's
/// Configuration page for why that's not literally bundled Chromium everywhere.
///
/// Called from the `open_result`/`close_builtin_browser` commands, both
/// `async fn` - see `open_position_picker`'s doc comment for why: creating
/// or navigating a window from a *synchronous* command is documented to
/// deadlock on Windows (https://github.com/tauri-apps/wry/issues/583),
/// which is Julian's "in-app browser freezes the whole app" bug. Wrapped
/// in `run_on_main_thread_with_timeout` as a general failsafe on top of
/// that root-cause fix, so a slow/stuck main thread reports a clean error
/// after a few seconds instead of taking the whole app down with it.
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
        .replace("__THEME_VARS__", &theme_vars_css(app));

    WebviewWindowBuilder::new(app, BROWSER_LABEL, WebviewUrl::External(parsed))
        .title("LUMA Browser")
        .inner_size(1100.0, 760.0)
        .min_inner_size(360.0, 320.0)
        .initialization_script(&toolbar_js)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// The current theme's colors, as `--luma-*: value;\n` declarations ready
/// to drop into a `:root { ... }` block - see builtin-browser-toolbar.js's
/// `__THEME_VARS__` placeholder.
///
/// The built-in browser window shows *external* page content, which never
/// loads any of Luma's own theme.css - so the toolbar injected on top of it
/// has no `var(--color-purple)` etc. to resolve against, and was always
/// stuck with one hardcoded look regardless of the theme actually picked
/// in Settings. Pulling the handful of colors the toolbar needs out of the
/// current theme's real CSS text and templating them straight into the
/// injected script's own `:root` block is what lets it actually match.
///
/// This is a small line-oriented scan for `--name: value;`, not a real CSS
/// parser - it doesn't need to be, since every theme.css (built-in or a
/// user's own custom one, see the wiki's Theming page) declares these once
/// in a single `:root { ... }` block in exactly this shape. Falls back to
/// Luma Default's own values for anything a theme doesn't define, so a
/// custom theme missing one of these can never leave the toolbar with a
/// broken/unset variable.
fn theme_vars_css(app: &AppHandle) -> String {
    // (the toolbar's variable name, the theme.css variable to read it
    // from, Luma Default's own value as the fallback)
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

    let mut out = String::new();
    for (toolbar_name, theme_var, fallback) in WANTED {
        let value = find_css_var(&css, theme_var).unwrap_or_else(|| (*fallback).to_string());
        out.push_str(&format!("  --{toolbar_name}: {value};\n"));
    }
    out
}

/// Finds the first `--name: value;` declaration in a block of CSS text and
/// returns `value`, trimmed. See `theme_vars_css` for why this is
/// deliberately not a full CSS parser.
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
