use crate::{config::LumaConfig, shortcuts, themes, window};
use std::io::{Read, Write};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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
    log_state: State<crate::logging::AppLog>,
    new_config: LumaConfig,
) -> Result<(), String> {
    let old_debug_logging = state.config.lock().unwrap().general.debug_logging;
    if new_config.general.debug_logging != old_debug_logging {
        log_state.set_enabled(&app, new_config.general.debug_logging);
    }

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

    let old_start_at_login = state.config.lock().unwrap().general.start_at_login;

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    {
        if new_config.general.start_at_login != old_start_at_login {
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
    }

    let old_main_window_size = state.config.lock().unwrap().window.main_window_size.clone();
    if new_config.window.main_window_size != old_main_window_size {
        window::apply_main_window_size(&app, &new_config.window.main_window_size);
    }

    let old_locale = state.config.lock().unwrap().general.locale.clone();
    if new_config.general.locale != old_locale {
        crate::tray::retext(&app, &new_config.general.locale);
    }

    *state.config.lock().unwrap() = new_config;

    let _ = app.emit("luma://config-changed", ());

    crate::logging::info(&app, "save_config: saved successfully");
    Ok(())
}

#[tauri::command]
pub fn list_themes(app: AppHandle) -> Vec<themes::ThemeInfo> {
    themes::list_themes(&app)
}

#[tauri::command]
pub fn list_plugins(app: AppHandle) -> Vec<crate::plugins::PluginInfo> {
    crate::plugins::list_plugins(&app)
}

#[tauri::command]
pub fn get_plugin_js(app: AppHandle, plugin_id: String) -> Result<String, String> {
    crate::plugins::js_for(&app, &plugin_id)
        .ok_or_else(|| format!("plugin '{plugin_id}' not found"))
}

#[tauri::command]
pub fn reveal_themes_folder(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = crate::config::themes_dir(&app);
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reveal_plugins_folder(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    crate::config::ensure_plugins_dir(&app);
    let dir = crate::config::plugins_dir(&app);
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn uninstall_theme(
    app: AppHandle,
    state: State<AppState>,
    theme_id: String,
) -> Result<(), String> {
    if crate::config::is_builtin_theme(&theme_id) {
        return Err("that theme ships with LUMA and can't be uninstalled".into());
    }

    let dir = crate::config::themes_dir(&app).join(&theme_id);
    if !dir.is_dir() {
        return Err("that theme isn't installed".into());
    }
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;

    let reverted_to_default = {
        let mut cfg = state.config.lock().unwrap();
        if cfg.appearance.theme == theme_id {
            cfg.appearance.theme = crate::config::DEFAULT_THEME_ID.to_string();
            crate::config::save(&app, &cfg)?;
            true
        } else {
            false
        }
    };

    crate::logging::info(
        &app,
        format!(
            "uninstalled theme '{theme_id}'{}",
            if reverted_to_default {
                " (was active - reverted to the default theme)"
            } else {
                ""
            }
        ),
    );

    let _ = app.emit("luma://config-changed", ());
    Ok(())
}

#[tauri::command]
pub fn uninstall_plugin(
    app: AppHandle,
    state: State<AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let dir = crate::config::plugins_dir(&app).join(&plugin_id);
    if !dir.is_dir() {
        return Err("that plugin isn't installed".into());
    }
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;

    {
        let mut cfg = state.config.lock().unwrap();
        cfg.plugins.enabled_plugins.retain(|id| id != &plugin_id);
        crate::config::save(&app, &cfg)?;
    }

    crate::logging::info(&app, format!("uninstalled plugin '{plugin_id}'"));

    let _ = app.emit("luma://config-changed", ());
    Ok(())
}

#[tauri::command]
pub fn set_plugin_enabled(
    app: AppHandle,
    state: State<AppState>,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    {
        let mut cfg = state.config.lock().unwrap();
        let has = cfg
            .plugins
            .enabled_plugins
            .iter()
            .any(|id| id == &plugin_id);
        if enabled && !has {
            cfg.plugins.enabled_plugins.push(plugin_id.clone());
        } else if !enabled && has {
            cfg.plugins.enabled_plugins.retain(|id| id != &plugin_id);
        }
        crate::config::save(&app, &cfg)?;
    }

    crate::logging::info(
        &app,
        format!(
            "plugin '{plugin_id}' {}",
            if enabled { "enabled" } else { "disabled" }
        ),
    );

    let _ = app.emit("luma://config-changed", ());
    Ok(())
}

#[tauri::command]
pub fn list_locales(app: AppHandle) -> Vec<crate::locales::LocaleInfo> {
    crate::locales::list_locales(&app)
}

#[tauri::command]
pub fn get_locale_strings(
    app: AppHandle,
    locale_id: String,
) -> std::collections::BTreeMap<String, String> {
    crate::locales::strings_for(&app, &locale_id)
}

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

#[tauri::command]
pub fn get_engines(app: AppHandle, state: State<AppState>) -> Result<serde_json::Value, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(ENGINES_JSON).map_err(|e| e.to_string())?;
    let cfg = state.config.lock().unwrap().clone();
    let custom_count = cfg.search.custom_engines.len();

    if let Some(arr) = value.get_mut("engines").and_then(|v| v.as_array_mut()) {
        arr.retain(|e| {
            e["name"]
                .as_str()
                .map(|name| cfg.search.enabled_builtin_engines.iter().any(|e| e == name))
                .unwrap_or(false)
        });

        crate::logging::debug(
            &app,
            format!(
                "get_engines: {} built-in enabled, {custom_count} custom",
                arr.len()
            ),
        );

        for engine in cfg.search.custom_engines {
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

        for plugin in crate::plugins::list_plugins(&app) {
            if !cfg
                .plugins
                .enabled_plugins
                .iter()
                .any(|id| id == &plugin.id)
            {
                continue;
            }
            for bang in &plugin.bangs {
                arr.push(serde_json::json!({
                    "name": bang.name,
                    "action": "",
                    "bang": bang.word,
                    "local": true,
                    "placeholder": plugin.name,
                    "plugin_id": plugin.id,
                }));
            }
        }

        if cfg.general.frecency_ranking {
            let usage = cfg.frecency.bang_usage.clone();
            let count_of = |e: &serde_json::Value| -> u64 {
                e["bang"]
                    .as_str()
                    .and_then(|b| usage.get(&b.to_lowercase()))
                    .copied()
                    .unwrap_or(0)
            };
            arr.sort_by_key(|e| std::cmp::Reverse(count_of(e)));
        }
    }

    Ok(value)
}

#[tauri::command]
pub fn record_bang_usage(
    app: AppHandle,
    state: State<AppState>,
    bang: String,
) -> Result<(), String> {
    let bang = bang
        .trim()
        .trim_start_matches('!')
        .trim_start_matches('@')
        .to_lowercase();
    if bang.is_empty() {
        return Ok(());
    }
    let mut cfg = state.config.lock().unwrap();
    *cfg.frecency.bang_usage.entry(bang).or_insert(0) += 1;
    crate::config::save(&app, &cfg)
}

#[tauri::command]
pub fn list_all_builtin_engines() -> Result<serde_json::Value, String> {
    let value: serde_json::Value = serde_json::from_str(ENGINES_JSON).map_err(|e| e.to_string())?;
    Ok(value.get("engines").cloned().unwrap_or_default())
}

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

    let added_bang = bang.clone();
    let added_name = name.clone();
    cfg.search.custom_engines.push(crate::config::CustomEngine {
        name,
        action,
        bang,
        placeholder: engine.placeholder.trim().to_string(),
    });
    let total = cfg.search.custom_engines.len();
    crate::config::save(&app, &cfg)?;
    crate::logging::debug(
        &app,
        format!(
            "add_custom_engine: added '{added_name}' (!{added_bang}), {total} custom engines now"
        ),
    );
    let _ = app.emit("luma://config-changed", ());
    Ok(())
}

#[tauri::command]
pub fn remove_custom_engine(
    app: AppHandle,
    state: State<AppState>,
    name: String,
) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    cfg.search.custom_engines.retain(|e| e.name != name);
    crate::config::save(&app, &cfg)?;
    crate::logging::debug(&app, format!("remove_custom_engine: removed '{name}'"));
    let _ = app.emit("luma://config-changed", ());
    Ok(())
}

#[tauri::command]
pub fn set_builtin_engine_enabled(
    app: AppHandle,
    state: State<AppState>,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    let already = cfg
        .search
        .enabled_builtin_engines
        .iter()
        .any(|e| e == &name);
    if enabled && !already {
        cfg.search.enabled_builtin_engines.push(name.clone());
    } else if !enabled && already {
        cfg.search.enabled_builtin_engines.retain(|e| e != &name);
    }
    crate::config::save(&app, &cfg)?;
    crate::logging::debug(
        &app,
        format!("set_builtin_engine_enabled: '{name}' -> {enabled}"),
    );
    let _ = app.emit("luma://config-changed", ());
    Ok(())
}

#[tauri::command]
pub fn remove_custom_app(
    app: AppHandle,
    state: State<AppState>,
    name: String,
) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    cfg.search.custom_apps.retain(|a| a.name != name);
    crate::config::save(&app, &cfg)?;
    crate::logging::debug(&app, format!("remove_custom_app: removed '{name}'"));
    let _ = app.emit("luma://config-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn search_local(app: AppHandle, query: String) -> Result<(), String> {
    let _ = &app;
    let query = query.trim();
    if query.is_empty() {
        return Err("nothing to search for".into());
    }

    #[cfg(target_os = "windows")]
    {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        if home.trim().is_empty() {
            return Err("could not find your home folder".into());
        }

        let uri = format!(
            "search-ms:query={}&crumb=location:{}",
            percent_encode_for_uri(query),
            percent_encode_for_uri(&home),
        );
        crate::logging::info(
            &app,
            format!("search_local: opening OS search for {query:?}"),
        );

        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let uri_h = HSTRING::from(uri.as_str());
        let verb_h = HSTRING::from("open");
        let result = unsafe {
            ShellExecuteW(
                None,
                &verb_h,
                &uri_h,
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };

        let code = result.0 as isize;
        if code <= 32 {
            return Err(format!("couldn't open Windows Search (error code {code})"));
        }
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

#[cfg(target_os = "windows")]
fn percent_encode_for_uri(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub const APP_NOT_FOUND: &str = "__LUMA_APP_NOT_FOUND__";

#[tauri::command]
pub fn open_app(app: AppHandle, state: State<AppState>, query: String) -> Result<(), String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("nothing to open".into());
    }

    let saved = state
        .config
        .lock()
        .unwrap()
        .search
        .custom_apps
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(query))
        .cloned();

    if let Some(entry) = saved {
        crate::logging::debug(
            &app,
            format!(
                "open_app: '{query}' matched a saved custom app at {:?}",
                entry.path
            ),
        );
        return launch_app_path(&entry.path);
    }

    let result = find_and_launch_app(query).map_err(|_| APP_NOT_FOUND.to_string());
    crate::logging::debug(
        &app,
        format!(
            "open_app: '{query}' auto-detect {}",
            if result.is_ok() {
                "found and launched"
            } else {
                "not found"
            }
        ),
    );
    result
}

#[tauri::command]
pub fn pick_app_for(app: AppHandle, state: State<AppState>, name: String) -> Result<(), String> {
    use tauri_plugin_dialog::DialogExt;

    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("nothing to open".into());
    }

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer.exe")
            .arg("shell:appsfolder")
            .spawn();
    }

    let locale = state.config.lock().unwrap().general.locale.clone();
    let title_fmt = crate::locales::strings_for(&app, &locale)
        .get("dialog.pick_app_title")
        .cloned()
        .unwrap_or_else(|| "Select the app for \"{0}\"".to_string());
    let picker = app
        .dialog()
        .file()
        .set_title(title_fmt.replace("{0}", &name));
    #[cfg(target_os = "windows")]
    let picker = picker.add_filter("Applications", &["exe"]);
    #[cfg(target_os = "macos")]
    let picker = picker.add_filter("Applications", &["app"]);

    let picked = picker
        .blocking_pick_file()
        .ok_or_else(|| "no file selected".to_string())?;
    let path = picked
        .into_path()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    {
        let mut cfg = state.config.lock().unwrap();
        cfg.search
            .custom_apps
            .retain(|a| !a.name.eq_ignore_ascii_case(&name));
        cfg.search.custom_apps.push(crate::config::CustomApp {
            name: name.clone(),
            path: path.clone(),
        });
        crate::config::save(&app, &cfg)?;
    }
    let _ = app.emit("luma://config-changed", ());

    launch_app_path(&path)
}

fn launch_app_path(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", path])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        if path.ends_with(".app") {
            std::process::Command::new("open").arg(path).spawn()
        } else {
            std::process::Command::new(path).spawn()
        }
        .map(|_| ())
        .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
        Err("opening apps isn't supported on this platform yet".into())
    }
}

#[cfg(target_os = "windows")]
fn find_and_launch_app(query: &str) -> Result<(), String> {
    let query_lower = query.to_lowercase();
    let roots = [
        std::env::var("ProgramData")
            .ok()
            .map(|p| format!("{p}\\Microsoft\\Windows\\Start Menu\\Programs")),
        std::env::var("AppData")
            .ok()
            .map(|p| format!("{p}\\Microsoft\\Windows\\Start Menu\\Programs")),
    ];

    for root in roots.into_iter().flatten() {
        if let Some(hit) = find_shortcut(std::path::Path::new(&root), &query_lower) {
            return std::process::Command::new("cmd")
                .args(["/C", "start", "", &hit.to_string_lossy()])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string());
        }
    }

    if let Some(app_id) = windows_find_start_app(&query_lower) {
        return std::process::Command::new("explorer.exe")
            .arg(format!("shell:appsfolder\\{app_id}"))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string());
    }

    Err("not found".into())
}

#[cfg(target_os = "windows")]
fn windows_find_start_app(query_lower: &str) -> Option<String> {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-StartApps | ForEach-Object { \"$($_.Name)|$($_.AppID)\" }",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| {
            let (name, app_id) = line.split_once('|')?;
            name.trim()
                .to_lowercase()
                .contains(query_lower)
                .then(|| app_id.trim().to_string())
        })
}

#[cfg(target_os = "windows")]
fn find_shortcut(dir: &std::path::Path, query_lower: &str) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(hit) = find_shortcut(&path, query_lower) {
                return Some(hit);
            }
            continue;
        }
        let is_lnk = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("lnk"));
        if !is_lnk {
            continue;
        }
        let matches = path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|stem| stem.to_lowercase().contains(query_lower));
        if matches {
            return Some(path);
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn find_and_launch_app(query: &str) -> Result<(), String> {
    let status = std::process::Command::new("open")
        .args(["-a", query])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("not found".into())
    }
}

#[cfg(target_os = "linux")]
fn find_and_launch_app(query: &str) -> Result<(), String> {
    let query_lower = query.to_lowercase();
    let mut dirs = vec![
        "/usr/share/applications".to_string(),
        "/usr/local/share/applications".to_string(),
        "/var/lib/snapd/desktop/applications".to_string(),
        "/var/lib/flatpak/exports/share/applications".to_string(),
    ];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(format!("{home}/.local/share/applications"));
        dirs.push(format!(
            "{home}/.local/share/flatpak/exports/share/applications"
        ));
    }

    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };
            let name = contents.lines().find_map(|l| l.strip_prefix("Name="));
            let Some(name) = name else { continue };
            if !name.to_lowercase().contains(&query_lower) {
                continue;
            }
            let exec_line = contents.lines().find_map(|l| l.strip_prefix("Exec="));
            let Some(exec_line) = exec_line else { continue };
            let Some((program, args)) = desktop_exec_command(exec_line) else {
                continue;
            };
            return std::process::Command::new(program)
                .args(args)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string());
        }
    }
    Err("not found".into())
}

#[cfg(target_os = "linux")]
fn desktop_exec_command(exec_line: &str) -> Option<(String, Vec<String>)> {
    let is_field_code = |tok: &str| {
        matches!(
            tok,
            "%f" | "%F"
                | "%u"
                | "%U"
                | "%d"
                | "%D"
                | "%n"
                | "%N"
                | "%i"
                | "%c"
                | "%k"
                | "%v"
                | "%m"
        )
    };
    let mut tokens = exec_line
        .split_whitespace()
        .filter(|t| !is_field_code(t) && *t != "@@u" && *t != "@@U" && *t != "@@")
        .map(|t| t.replace("%%", "%"));
    let program = tokens.next()?;
    if program.is_empty() {
        return None;
    }
    Some((program, tokens.collect()))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn find_and_launch_app(_query: &str) -> Result<(), String> {
    Err("opening apps isn't supported on this platform yet".into())
}

#[tauri::command]
pub fn get_theme_css(app: AppHandle, theme_id: String) -> Result<String, String> {
    let result =
        themes::css_for(&app, &theme_id).ok_or_else(|| format!("theme '{theme_id}' not found"));
    crate::logging::debug(
        &app,
        format!(
            "get_theme_css: '{theme_id}' -> {}",
            match &result {
                Ok(css) => format!("{} bytes", css.len()),
                Err(e) => format!("error: {e}"),
            }
        ),
    );
    result
}

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
        cfg.general.show_spotlight_branding,
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

#[tauri::command]
pub async fn open_position_picker(app: AppHandle) -> Result<(), String> {
    window::open_position_picker(&app)
}

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

#[tauri::command]
pub async fn cancel_position_pick(app: AppHandle) -> Result<(), String> {
    window::cancel_position_pick(&app)
}

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

#[tauri::command]
pub fn reset_to_defaults(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let defaults = LumaConfig::default();
    crate::config::save(&app, &defaults)?;
    *state.config.lock().unwrap() = defaults;
    crate::logging::info(
        &app,
        "reset_to_defaults: config.toml reset to defaults".to_string(),
    );
    let _ = app.emit("luma://config-changed", ());
    Ok(())
}

#[tauri::command]
pub fn clear_browsing_data(app: AppHandle) -> Result<(), String> {
    let win = window::main_window(&app)
        .or_else(|| window::spotlight_window(&app))
        .ok_or("no Luma window is open to clear data from")?;
    win.clear_all_browsing_data()
        .map_err(|e| format!("couldn't clear browsing data: {e}"))?;
    crate::logging::info(
        &app,
        "clear_browsing_data: cleared cookies/cache/history".to_string(),
    );
    Ok(())
}

#[tauri::command]
pub fn uninstall_app(app: AppHandle) -> Result<(), String> {
    crate::logging::info(&app, "uninstall_app: starting uninstall".to_string());
    crate::uninstall::run()
}

/// Newest-first list of files directly inside `dir`, sorted by mtime.
fn backups_newest_first(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(std::path::PathBuf, std::time::SystemTime)> = entries
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((e.path(), modified))
        })
        .collect();
    files.sort_by_key(|(_, modified)| std::cmp::Reverse(*modified));
    files.into_iter().map(|(path, _)| path).collect()
}

/// Keeps only the `keep` most recently modified files in `dir`, deleting the rest.
fn prune_backups(dir: &std::path::Path, keep: usize) {
    for old in backups_newest_first(dir).into_iter().skip(keep) {
        let _ = std::fs::remove_file(old);
    }
}

fn backup_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

const BACKUPS_TO_KEEP: usize = 6;

#[tauri::command]
pub fn backup_config(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let text = {
        let cfg = state.config.lock().unwrap();
        toml::to_string_pretty(&*cfg).map_err(|e| e.to_string())?
    };

    let desktop_path = crate::config::desktop_dir(&app).join("luma-config-backup.toml");
    std::fs::write(&desktop_path, &text).map_err(|e| e.to_string())?;

    let backups_dir = crate::config::backups_dir(&app);
    std::fs::create_dir_all(&backups_dir).map_err(|e| e.to_string())?;
    let archive_path = backups_dir.join(format!("config_{}.toml", backup_timestamp()));
    std::fs::write(&archive_path, &text).map_err(|e| e.to_string())?;
    prune_backups(&backups_dir, BACKUPS_TO_KEEP);

    crate::logging::info(
        &app,
        format!("backup_config: wrote {desktop_path:?} and {archive_path:?}"),
    );
    Ok(())
}

#[tauri::command]
pub fn backup_full(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let config_text = {
        let cfg = state.config.lock().unwrap();
        toml::to_string_pretty(&*cfg).map_err(|e| e.to_string())?
    };

    let mut buf: Vec<u8> = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::SimpleFileOptions::default();

        zip.start_file("config.toml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(config_text.as_bytes())
            .map_err(|e| e.to_string())?;

        add_dir_to_zip(
            &mut zip,
            &crate::config::plugins_dir(&app),
            "plugins",
            options,
        )?;
        add_dir_to_zip(
            &mut zip,
            &crate::config::themes_dir(&app),
            "themes",
            options,
        )?;

        zip.finish().map_err(|e| e.to_string())?;
    }

    let desktop_path = crate::config::desktop_dir(&app).join("luma-full-backup.zip");
    std::fs::write(&desktop_path, &buf).map_err(|e| e.to_string())?;

    let backups_dir = crate::config::backups_dir(&app);
    std::fs::create_dir_all(&backups_dir).map_err(|e| e.to_string())?;
    let archive_path = backups_dir.join(format!("full_{}.zip", backup_timestamp()));
    std::fs::write(&archive_path, &buf).map_err(|e| e.to_string())?;
    prune_backups(&backups_dir, BACKUPS_TO_KEEP);

    crate::logging::info(
        &app,
        format!("backup_full: wrote {desktop_path:?} and {archive_path:?}"),
    );
    Ok(())
}

fn add_dir_to_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    dir: &std::path::Path,
    prefix: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in walk_dir_flat(dir) {
        let rel = entry.strip_prefix(dir).map_err(|e| e.to_string())?;
        let zip_path = format!("{prefix}/{}", rel.to_string_lossy().replace('\\', "/"));
        if entry.is_dir() {
            continue;
        }
        let data = std::fs::read(&entry).map_err(|e| e.to_string())?;
        zip.start_file(zip_path, options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&data).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn walk_dir_flat(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    fn walk(d: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(d) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    walk(dir, &mut out);
    out
}

#[tauri::command]
pub fn restore_backup(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("Luma backup", &["zip", "toml"])
        .blocking_pick_file()
        .ok_or_else(|| "no file selected".to_string())?;
    let path = picked.into_path().map_err(|e| e.to_string())?;

    let is_zip = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"));

    let new_cfg: LumaConfig = if is_zip {
        restore_from_zip(&app, &path)?
    } else {
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        toml::from_str(&text).map_err(|e| e.to_string())?
    };

    crate::config::save(&app, &new_cfg)?;
    *state.config.lock().unwrap() = new_cfg;
    let _ = app.emit("luma://config-changed", ());
    crate::logging::info(&app, format!("restore_backup: restored from {path:?}"));
    Ok(())
}

fn restore_from_zip(app: &AppHandle, path: &std::path::Path) -> Result<LumaConfig, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let mut config_text: Option<String> = None;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if name.ends_with('/') {
            continue;
        }
        if name == "config.toml" {
            let mut text = String::new();
            entry.read_to_string(&mut text).map_err(|e| e.to_string())?;
            config_text = Some(text);
        } else if let Some(rel) = name.strip_prefix("plugins/") {
            extract_zip_entry(&mut entry, &crate::config::plugins_dir(app).join(rel))?;
        } else if let Some(rel) = name.strip_prefix("themes/") {
            extract_zip_entry(&mut entry, &crate::config::themes_dir(app).join(rel))?;
        }
    }

    let config_text = config_text.ok_or("that zip doesn't contain a config.toml")?;
    toml::from_str(&config_text).map_err(|e| e.to_string())
}

fn extract_zip_entry(entry: &mut zip::read::ZipFile, dest: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut data = Vec::new();
    entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
    std::fs::write(dest, data).map_err(|e| e.to_string())
}
