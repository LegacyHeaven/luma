use crate::{config::LumaConfig, shortcuts, themes, window};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

// Windows only: every `!open`/app-launch path below that has to go through a
// console-subsystem helper (`cmd.exe`, `powershell.exe`) rather than
// launching the target GUI app directly needs this, or Windows allocates a
// brand new console window for that child process - since Luma itself is a
// GUI app with no console of its own to inherit - and it flashes on screen
// for a moment even though the helper only lives long enough to hand off to
// the real target and exit. explorer.exe elsewhere in this file doesn't need
// it: it's a GUI-subsystem executable and never allocates a console at all.
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

    let old_start_at_login = state.config.lock().unwrap().general.start_at_login;

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    {
        // Only touch the OS-level autostart registration when the setting
        // actually changed. Calling enable()/disable() unconditionally on
        // every save (the previous behavior) meant disable() ran even when
        // autostart had never been enabled in the first place - on Windows
        // that's a registry delete-value call with nothing to delete, which
        // the underlying `auto-launch` crate surfaces as a hard error
        // ("could not update start-at-login: ... os error 2") on every
        // single settings save, not just when the user actually toggled it.
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
pub fn reveal_themes_folder(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = crate::config::themes_dir(&app);
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
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
pub fn get_engines(state: State<AppState>) -> Result<serde_json::Value, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(ENGINES_JSON).map_err(|e| e.to_string())?;
    let cfg = state.config.lock().unwrap().clone();

    if let Some(arr) = value.get_mut("engines").and_then(|v| v.as_array_mut()) {
        arr.retain(|e| {
            e["name"]
                .as_str()
                .map(|name| cfg.search.enabled_builtin_engines.iter().any(|e| e == name))
                .unwrap_or(false)
        });

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
    }

    Ok(value)
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
        cfg.search.enabled_builtin_engines.push(name);
    } else if !enabled && already {
        cfg.search.enabled_builtin_engines.retain(|e| e != &name);
    }
    crate::config::save(&app, &cfg)?;
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
    let _ = app.emit("luma://config-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn search_mypc(app: AppHandle, query: String) -> Result<(), String> {
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

        // Hand this off to Windows' own indexed search - the exact same
        // index behind the Start Menu's and Explorer's own search boxes -
        // via the `search-ms:` URI scheme, instead of walking the
        // filesystem by hand ourselves. A manual walk (what this used to
        // do) can only ever match on filenames, never file *content* the
        // real index already covers, is slow doing synchronous disk I/O on
        // Luma's own main thread, and - critically, this is why it kept
        // not working - had to give up after a fixed 3-second/40,000-entry
        // budget, which a dev machine's home folder (node_modules, git
        // clones, build output, ...) can blow through long before ever
        // reaching whatever the user actually typed. `explorer.exe` opens
        // a normal, live-updating Explorer search-results window against
        // the real index instead, scoped to the user's own home folder via
        // the `crumb=location:` parameter (matching what this always
        // searched before - just done properly this time).
        let uri = format!(
            "search-ms:query={}&crumb=location:{}",
            percent_encode_for_uri(query),
            percent_encode_for_uri(&home),
        );
        crate::logging::info(
            &app,
            format!("search_mypc: opening OS search for {query:?}"),
        );

        // This used to just spawn `explorer.exe <uri>` as a plain child
        // process. Confirmed live that this does NOT reliably resolve the
        // search-ms: protocol at all: it popped Windows' "Open With"
        // chooser instead of a Search Results window. The reason is that
        // std::process::Command is a bare CreateProcess - it starts a new
        // explorer.exe, but never goes through the shell's own
        // protocol-handler resolution, which is what actually knows
        // search-ms: is owned by Explorer's search host (that mapping
        // lives in the registry under HKEY_CLASSES_ROOT\search-ms).
        // ShellExecuteW is the real API a Run dialog or a shell link uses
        // to open a URI like this: it looks the scheme up itself and hands
        // the URI to whatever's registered for it, which is the same
        // mechanism that makes `search-ms:` links work from anywhere else
        // in Windows. Calling it directly here instead of hoping
        // explorer.exe's own argv parsing special-cases the string is the
        // actual fix - not just another way of asking for the same thing.
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
        // ShellExecute's return value is an HINSTANCE-shaped legacy status
        // code, not a real handle: per Microsoft's own docs, anything
        // greater than 32 means success and everything else is an error
        // code - there's no richer error to extract out of this ancient
        // API than that.
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

// Minimal percent-encoding for building the `search-ms:` URI above - not
// pulling in the `percent-encoding` crate for one call site. Encodes every
// byte outside the small unreserved-characters set (RFC 3986), which is
// always safe even though `search-ms:` is Explorer's own URI scheme rather
// than a standard http(s) one; iterating by UTF-8 byte rather than by char
// means multi-byte characters (accented names, non-Latin scripts, etc. -
// all valid in a Windows path or search query) come out correctly as a run
// of individually-percent-encoded bytes.
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
pub fn open_app(state: State<AppState>, query: String) -> Result<(), String> {
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

    if let Some(app) = saved {
        return launch_app_path(&app.path);
    }

    find_and_launch_app(query).map_err(|_| APP_NOT_FOUND.to_string())
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
        // Pop open the same "every installed app" view Windows' own search
        // uses (Store/UWP apps included) so whatever open_app's automatic
        // detection couldn't find is easy to spot before falling back to a
        // plain file browse below. Best-effort - if explorer.exe isn't on
        // PATH for some reason, the file dialog below still works.
        let _ = std::process::Command::new("explorer.exe")
            .arg("shell:appsfolder")
            .spawn();
    }

    let picker = app
        .dialog()
        .file()
        .set_title(format!("Select the app for \"{name}\""));
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

    // The .lnk scan above misses anything that isn't a classic desktop
    // shortcut - Store/UWP apps (Calculator, Settings, Photos, Spotify's
    // UWP build, ...) don't have one anywhere on disk. Get-StartApps reads
    // the exact same catalog Windows' own Start menu search and the
    // shell:appsfolder view use, so this is the same "search everything
    // installed" behavior without reimplementing the shell namespace.
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
        // Snap and Flatpak both publish their own .desktop files instead of
        // registering with the traditional dirs above - without these, any
        // app installed either way is invisible to !open even though it
        // shows up in every desktop environment's app launcher.
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

// Parses a .desktop file's `Exec=` line into a program + argument list,
// dropping the freedesktop field codes (%f, %U, etc.) a launcher is
// supposed to fill in. Earlier this only kept the first whitespace-separated
// token, which happened to work for plain `Exec=firefox %u` entries but
// silently launched a no-op for anything wrapped in a runner - `Exec=flatpak
// run --branch=stable com.spotify.Client @@u %U@@` would spawn bare
// `flatpak` with no arguments and go nowhere.
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
    themes::css_for(&app, &theme_id).ok_or_else(|| format!("theme '{theme_id}' not found"))
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
