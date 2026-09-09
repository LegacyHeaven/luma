use crate::{config::LumaConfig, shortcuts, themes, window};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

pub struct AppState {
    pub config: Mutex<LumaConfig>,
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> LumaConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config(
    app: AppHandle,
    state: State<AppState>,
    new_config: LumaConfig,
) -> Result<(), String> {
    crate::logging::info(&app, format!("save_config called: {new_config:?}"));

    if let Err(err) = crate::config::save(&app, &new_config) {
        crate::logging::error(
            &app,
            format!("save_config: writing config.toml failed: {err}"),
        );
        return Err(err);
    }

    if let Err(err) = shortcuts::reregister(&app, &new_config.general.shortcut) {
        crate::logging::error(
            &app,
            format!("save_config: could not apply new shortcut: {err}"),
        );
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    {
        use tauri_plugin_autostart::ManagerExt;
        let autostart = app.autolaunch();
        let result = if new_config.general.start_at_login {
            autostart.enable()
        } else {
            autostart.disable()
        };
        if let Err(err) = result {
            crate::logging::warn(&app, format!("could not update start-at-login: {err}"));
        }
    }

    let old_main_window_size = state.config.lock().unwrap().window.main_window_size.clone();
    if new_config.window.main_window_size != old_main_window_size {
        window::apply_main_window_size(&app, &new_config.window.main_window_size);
    }

    *state.config.lock().unwrap() = new_config;

    // Lets any open window (chiefly the spotlight, which is created once
    // and never reloads) know it should re-fetch config/theme CSS and
    // re-apply it live - see main.js's "luma://config-changed" listener.
    let _ = app.emit("luma://config-changed", ());

    crate::logging::info(&app, "save_config: saved successfully");
    Ok(())
}

#[tauri::command]
pub fn list_themes(app: AppHandle) -> Vec<themes::ThemeInfo> {
    themes::list_themes(&app)
}

#[tauri::command]
pub fn reveal_themes_folder(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = crate::config::themes_dir(&app);
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// The engine catalog, compiled straight into the binary so a downloaded
/// `luma` executable has no separate resource file to lose track of.
const ENGINES_JSON: &str = include_str!("../resources/engines.json");

fn builtin_bangs() -> Vec<String> {
    let parsed: serde_json::Value = serde_json::from_str(ENGINES_JSON).unwrap_or_default();
    parsed["engines"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|e| e["bang"].as_str())
                .map(|b| b.to_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

/// The frontend fetches the engine catalog through this command rather than
/// a `<script src>`, so the same code path runs in `cargo tauri dev` and in
/// a release build. Merges in the user's own engines from Settings' "Search
/// engines" section (see config::CustomEngine) - marked `template: true` so
/// bangdeck.js knows to substitute `%s` rather than treat `action` as a
/// fixed base URL, and `user_added: true` so Settings can tell them apart
/// from the built-in catalog when rendering its "remove" list.
#[tauri::command]
pub fn get_engines(state: State<AppState>) -> Result<serde_json::Value, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(ENGINES_JSON).map_err(|e| e.to_string())?;
    let custom = state.config.lock().unwrap().search.custom_engines.clone();

    if let Some(arr) = value.get_mut("engines").and_then(|v| v.as_array_mut()) {
        for engine in custom {
            let placeholder = if engine.placeholder.trim().is_empty() {
                format!("search {}", engine.name)
            } else {
                engine.placeholder.clone()
            };
            arr.push(serde_json::json!({
                "name": engine.name,
                "action": engine.action,
                "bang": engine.bang,
                "placeholder": placeholder,
                "template": true,
                "user_added": true,
            }));
        }
    }

    Ok(value)
}

/// Settings' "Add engine" form. Deliberately picky about validation here
/// rather than in the frontend alone - config.toml can be hand-edited, and
/// a bad entry there would otherwise silently break every search that
/// falls through to the default engine.
#[tauri::command]
pub fn add_custom_engine(
    app: AppHandle,
    state: State<AppState>,
    engine: crate::config::CustomEngine,
) -> Result<(), String> {
    let name = engine.name.trim().to_string();
    let action = engine.action.trim().to_string();
    let bang = engine.bang.trim().trim_start_matches('!').to_lowercase();

    if name.is_empty() || action.is_empty() || bang.is_empty() {
        return Err("name, search URL, and bang are all required".into());
    }
    if !action.contains("%s") {
        return Err("the search URL needs a %s where your search text should go".into());
    }
    if !bang.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("bang words can only contain letters and numbers".into());
    }

    let mut cfg = state.config.lock().unwrap();

    if builtin_bangs().contains(&bang) || cfg.search.custom_engines.iter().any(|e| e.bang == bang) {
        return Err(format!("!{bang} is already used by another engine"));
    }
    if cfg
        .search
        .custom_engines
        .iter()
        .any(|e| e.name.eq_ignore_ascii_case(&name))
    {
        return Err(format!("you already have an engine named \"{name}\""));
    }

    cfg.search.custom_engines.push(crate::config::CustomEngine {
        name,
        action,
        bang,
        placeholder: engine.placeholder.trim().to_string(),
    });
    crate::config::save(&app, &cfg)?;
    let _ = app.emit("luma://config-changed", ());
    Ok(())
}

/// Settings' "remove" button next to a custom engine.
#[tauri::command]
pub fn remove_custom_engine(
    app: AppHandle,
    state: State<AppState>,
    name: String,
) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    cfg.search.custom_engines.retain(|e| e.name != name);
    crate::config::save(&app, &cfg)?;
    let _ = app.emit("luma://config-changed", ());
    Ok(())
}

/// `!mypc` - Julian's requested "OS integration" bang: search this
/// computer itself (files, and whatever else each platform's own search
/// facility indexes) instead of the web. There's no cross-platform API for
/// this, so each platform hands off to whatever it already has, rather
/// than Luma trying to maintain its own file index:
///
/// - Windows: opens Explorer's own federated search UI via the
///   `search-ms:` URI, pre-filled with the query - genuinely searches
///   files/folders across the indexed locations (Documents, Desktop, ...).
/// - macOS: there's no public API to pre-fill Spotlight's own search
///   field, so this queries Spotlight's index directly with `mdfind`
///   (fast, read-only) and reveals the best match in Finder.
/// - Linux: no standard desktop-search protocol to hook into, so this
///   does the same best-match-and-reveal thing as macOS with `find`.
///
/// Every branch only ever *launches* another already-installed program
/// (Explorer/Finder/the file manager) - Luma itself never reads file
/// contents or lists directories.
///
/// `async fn`: the macOS/Linux branches block on an external process
/// (`mdfind`/`find`) that can take a moment on a big home folder - async
/// keeps that off the main thread like every other command that might not
/// return instantly, rather than risking a smaller version of the same
/// "the whole app hangs" problem this session's other fixes went after.
#[tauri::command]
pub async fn search_mypc(query: String) -> Result<(), String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("nothing to search for".into());
    }

    #[cfg(target_os = "windows")]
    {
        let uri = format!("search-ms:query={}", percent_encode(query));
        std::process::Command::new("explorer.exe")
            .arg(uri)
            .spawn()
            .map_err(|e| format!("couldn't open Windows Search: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        reveal_best_match("mdfind", &["-onlyin", &home_dir()], query, "open", &["-R"])
    }

    #[cfg(target_os = "linux")]
    {
        reveal_best_match(
            "find",
            &[home_dir().as_str(), "-iname"],
            &format!("*{query}*"),
            "xdg-open",
            &[],
        )
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = query;
        Err("system search isn't supported on this platform yet".into())
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_default()
}

/// Shared by the macOS/Linux branches of search_mypc: runs a local search
/// command, takes its first result, and reveals/opens it with a second
/// command - never Luma's own process reading the file, just handing a
/// path to a tool the OS already provides.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn reveal_best_match(
    search_cmd: &str,
    search_args: &[&str],
    query: &str,
    reveal_cmd: &str,
    reveal_args: &[&str],
) -> Result<(), String> {
    let mut cmd = std::process::Command::new(search_cmd);
    cmd.args(search_args);
    cmd.arg(query);
    let output = cmd
        .output()
        .map_err(|e| format!("{search_cmd} failed: {e}"))?;

    let first_match = String::from_utf8_lossy(&output.stdout)
        .lines()
        .find(|l| !l.trim().is_empty())
        .map(|s| s.to_string());

    match first_match {
        Some(path) => {
            std::process::Command::new(reveal_cmd)
                .args(reveal_args)
                .arg(&path)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        None => Err(format!("nothing on your PC matched \"{query}\"")),
    }
}

/// Minimal percent-encoding for building the `search-ms:` URI above - only
/// the small fixed allow-list of characters that never need escaping in a
/// URI component are left alone; everything else (spaces, punctuation, and
/// non-ASCII, byte-by-byte, which is correct for UTF-8) is escaped. Avoids
/// pulling in a whole crate just for this one query parameter.
#[cfg(target_os = "windows")]
fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[tauri::command]
pub fn get_theme_css(app: AppHandle, theme_id: String) -> Result<String, String> {
    themes::css_for(&app, &theme_id).ok_or_else(|| format!("theme '{theme_id}' not found"))
}

/// Called by the frontend once it has resolved a `!bang`/plain query into a
/// concrete URL (see src/vendor/engine/bangdeck.js) - decides whether to hand
/// it to the system browser or Luma's built-in browser window, per config.
///
/// `async fn`, not a plain fn: the "builtin" path can create/navigate a
/// window (see window::open_in_builtin_browser), and doing that from a
/// *synchronous* command is a documented Windows/WebView2 deadlock
/// (https://github.com/tauri-apps/wry/issues/583) - this was Julian's
/// "in-app browser freezes the whole app, and even the debug log stops
/// responding" bug. Being async runs this on the async runtime instead of
/// blocking the main thread, so the main-thread hop window creation needs
/// can actually complete.
#[tauri::command]
pub async fn open_result(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    from_spotlight: bool,
) -> Result<(), String> {
    let mode = state.config.lock().unwrap().general.browser_mode.clone();
    crate::logging::info(
        &app,
        format!("open_result: url={url:?} mode={mode:?} from_spotlight={from_spotlight}"),
    );

    let result = match mode.as_str() {
        "builtin" => window::open_in_builtin_browser(&app, &url),
        _ => open_in_system_browser(app.clone(), url),
    };

    if let Err(err) = &result {
        crate::logging::error(&app, format!("open_result: failed to open: {err}"));
    } else {
        crate::logging::info(&app, "open_result: opened successfully");
    }

    if from_spotlight {
        window::hide_spotlight(&app);
    }

    result
}

#[tauri::command]
pub fn open_in_system_browser(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    crate::logging::info(
        &app,
        format!("open_in_system_browser: calling opener.open_url({url:?})"),
    );
    match app.opener().open_url(url, None::<&str>) {
        Ok(()) => {
            crate::logging::info(&app, "open_in_system_browser: opener.open_url returned Ok");
            Ok(())
        }
        Err(err) => {
            crate::logging::error(
                &app,
                format!("open_in_system_browser: opener.open_url returned Err: {err}"),
            );
            Err(err.to_string())
        }
    }
}

#[tauri::command]
pub async fn close_builtin_browser(app: AppHandle) -> Result<(), String> {
    window::close_builtin_browser(&app)
}

#[tauri::command]
pub fn toggle_spotlight(app: AppHandle, state: State<AppState>) {
    let cfg = state.config.lock().unwrap().clone();
    window::toggle_spotlight(
        &app,
        cfg.window.spotlight_width as f64,
        &cfg.window.spotlight_position,
        cfg.window.spotlight_custom_x,
        cfg.window.spotlight_custom_y,
    );
}

#[tauri::command]
pub fn hide_spotlight(app: AppHandle) {
    window::hide_spotlight(&app);
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) {
    window::show_main_window(&app);
}

/// Settings' "Pick position" button - opens the fullscreen click-to-choose
/// overlay (see window::open_position_picker). The overlay reports back
/// through `report_spotlight_position` or `cancel_position_pick`, never
/// directly - it has no config access of its own.
///
/// `async fn` - see window::open_position_picker's doc comment: creating
/// this window from a synchronous command is the documented Windows
/// deadlock (wry#583) behind "the pick position fails the same way as the
/// in-app browser".
#[tauri::command]
pub async fn open_position_picker(app: AppHandle) -> Result<(), String> {
    window::open_position_picker(&app)
}

/// The position-picker overlay calls this when the user clicks a spot.
/// `x_frac`/`y_frac` are fractions (0.0-1.0) of the primary monitor's size.
/// Saves straight to config and closes the overlay - per Julian's spec,
/// picking a spot *is* saving it, with no separate "Save" step - then lets
/// the spotlight (and Settings, if open) know the position changed.
#[tauri::command]
pub async fn report_spotlight_position(
    app: AppHandle,
    state: State<'_, AppState>,
    x_frac: f64,
    y_frac: f64,
) -> Result<(), String> {
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.window.spotlight_position = "custom".into();
        cfg.window.spotlight_custom_x = Some(x_frac.clamp(0.0, 1.0));
        cfg.window.spotlight_custom_y = Some(y_frac.clamp(0.0, 1.0));
        crate::config::save(&app, &cfg)?;
    }
    window::close_position_picker(&app)?;
    let _ = app.emit("luma://config-changed", ());
    crate::logging::info(
        &app,
        format!("report_spotlight_position: saved custom position ({x_frac:.4}, {y_frac:.4})"),
    );
    Ok(())
}

/// Escape on the position-picker overlay - closes it without touching
/// config. Still emits config-changed (a harmless no-op refresh) so
/// Settings' "Pick position" button re-enables itself either way - it has
/// no other way to learn the overlay is gone.
#[tauri::command]
pub async fn cancel_position_pick(app: AppHandle) -> Result<(), String> {
    window::close_position_picker(&app)?;
    let _ = app.emit("luma://config-changed", ());
    Ok(())
}

/// Settings' "Reset to default" button for the spotlight position - back
/// to always-centered, discarding any picked point.
#[tauri::command]
pub fn reset_spotlight_position(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    cfg.window.spotlight_position = "center".into();
    cfg.window.spotlight_custom_x = None;
    cfg.window.spotlight_custom_y = None;
    crate::config::save(&app, &cfg)?;
    let _ = app.emit("luma://config-changed", ());
    Ok(())
}
