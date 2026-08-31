use crate::{config::LumaConfig, shortcuts, themes, window};
use std::sync::Mutex;
use tauri::{AppHandle, State};

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
    crate::config::save(&app, &new_config)?;

    if let Err(err) = shortcuts::reregister(&app, &new_config.general.shortcut) {
        eprintln!("luma: could not apply new shortcut: {err}");
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
            eprintln!("luma: could not update start-at-login: {err}");
        }
    }

    *state.config.lock().unwrap() = new_config;
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

/// The frontend fetches the engine catalog through this command rather than
/// a `<script src>`, so the same code path runs in `cargo tauri dev` and in
/// a release build.
#[tauri::command]
pub fn get_engines() -> Result<serde_json::Value, String> {
    serde_json::from_str(ENGINES_JSON).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_theme_css(app: AppHandle, theme_id: String) -> Result<String, String> {
    themes::css_for(&app, &theme_id).ok_or_else(|| format!("theme '{theme_id}' not found"))
}

/// Called by the frontend once it has resolved a `!bang`/plain query into a
/// concrete URL (see src/vendor/engine/bangdeck.js) - decides whether to hand
/// it to the system browser or Luma's built-in browser window, per config.
#[tauri::command]
pub fn open_result(
    app: AppHandle,
    state: State<AppState>,
    url: String,
    from_spotlight: bool,
) -> Result<(), String> {
    let mode = state.config.lock().unwrap().general.browser_mode.clone();

    match mode.as_str() {
        "builtin" => window::open_in_builtin_browser(&app, &url)?,
        _ => open_in_system_browser(app.clone(), url)?,
    }

    if from_spotlight {
        window::hide_spotlight(&app);
    }

    Ok(())
}

#[tauri::command]
pub fn open_in_system_browser(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_builtin_browser(app: AppHandle) {
    window::close_builtin_browser(&app);
}

#[tauri::command]
pub fn toggle_spotlight(app: AppHandle, state: State<AppState>) {
    let cfg = state.config.lock().unwrap().clone();
    window::toggle_spotlight(
        &app,
        cfg.window.spotlight_width as f64,
        &cfg.window.spotlight_position,
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
