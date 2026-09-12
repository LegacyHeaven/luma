use serde::{Deserialize, Serialize};
use std::fs;
#[cfg(target_os = "windows")]
use std::path::Path;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub shortcut: String,

    pub browser_mode: String,

    pub default_engine: String,
    pub start_at_login: bool,

    pub close_spotlight_on_blur: bool,

    pub debug_logging: bool,

    pub check_for_updates: bool,

    pub show_spotlight_branding: bool,

    pub disable_animations: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            shortcut: "Alt+Space".into(),
            browser_mode: "system".into(),
            default_engine: "Google".into(),
            start_at_login: false,
            close_spotlight_on_blur: true,
            debug_logging: false,
            check_for_updates: true,
            show_spotlight_branding: false,
            disable_animations: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    pub theme: String,

    pub custom_css: String,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "luma-default".into(),
            custom_css: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub spotlight_width: u32,

    pub spotlight_position: String,

    pub spotlight_custom_x: Option<f64>,
    pub spotlight_custom_y: Option<f64>,

    pub main_window_size: String,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            spotlight_width: 640,
            spotlight_position: "center".into(),
            spotlight_custom_x: None,
            spotlight_custom_y: None,
            main_window_size: "default".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomEngine {
    pub name: String,

    pub action: String,

    pub bang: String,
    #[serde(default)]
    pub placeholder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomApp {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchConfig {
    pub custom_engines: Vec<CustomEngine>,

    pub custom_apps: Vec<CustomApp>,

    pub enabled_builtin_engines: Vec<String>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            custom_engines: Vec::new(),
            custom_apps: Vec::new(),
            enabled_builtin_engines: vec!["Google".into(), "MyPC".into(), "Open".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LumaConfig {
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub window: WindowConfig,
    pub search: SearchConfig,
}

// Windows: keep config.toml / themes / everything else beside the installed
// luma.exe itself instead of tucked away in %AppData%\Roaming - so anyone
// who goes looking for it by browsing to wherever Luma is installed finds
// it right there, no knowledge of (or need to un-hide) AppData required.
// Only falls back to the normal per-user app-config location if the exe's
// own folder can't be resolved or turns out not to be writable (e.g. Luma
// ends up installed somewhere that needs admin rights, like Program Files)
// - a "portable" config dir Luma can't actually write to would be worse
// than not being portable at all. Checked once per run and cached: nothing
// about an already-running process's own exe path or that folder's
// writability is going to change out from under it mid-session, and
// config_dir() is called often enough (every load/save) that redoing a
// filesystem probe each time would be wasteful.
#[cfg(target_os = "windows")]
fn portable_config_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.to_path_buf();
    fs::create_dir_all(&dir).ok()?;
    let probe = dir.join(".luma-write-test");
    fs::write(&probe, b"").ok()?;
    let _ = fs::remove_file(&probe);
    Some(dir)
}

// One-time migration for anyone who already has a config from before this
// moved beside the exe: if the new location doesn't have a config.toml yet
// but the old %AppData%\Roaming one does, copy config.toml and the themes
// folder over rather than silently falling back to defaults and losing
// every setting (custom engines/apps, spotlight position and width,
// disabled-animations, custom CSS, ...) the first time this version runs.
// Cheap to call repeatedly - the `new_config.exists()` check up front makes
// every call after the first (successful or not) a single stat call.
// Best-effort and never fatal: if anything here fails or there's nothing to
// migrate, load() falling back to defaults afterwards is the exact same
// behavior a missing/unreadable config has always had.
#[cfg(target_os = "windows")]
fn migrate_from_old_location(app: &AppHandle, new_dir: &Path) {
    let new_config = new_dir.join("config.toml");
    if new_config.exists() {
        return;
    }
    let Ok(old_dir) = app.path().app_config_dir() else {
        return;
    };
    if old_dir == new_dir {
        return;
    }
    let old_config = old_dir.join("config.toml");
    if !old_config.exists() {
        return;
    }

    crate::logging::info(
        app,
        format!("config: migrating from {old_dir:?} to {new_dir:?}"),
    );
    if let Err(err) = fs::copy(&old_config, &new_config) {
        crate::logging::error(app, format!("config: migration copy failed: {err}"));
        return;
    }

    let old_themes = old_dir.join("themes");
    let new_themes = new_dir.join("themes");
    if old_themes.is_dir() && !new_themes.exists() {
        if let Err(err) = copy_dir_all(&old_themes, &new_themes) {
            crate::logging::error(app, format!("config: theme migration failed: {err}"));
        }
    }
}

#[cfg(target_os = "windows")]
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dest_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

pub fn config_dir(app: &AppHandle) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        use std::sync::OnceLock;
        static PORTABLE_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
        if let Some(dir) = PORTABLE_DIR.get_or_init(portable_config_dir) {
            migrate_from_old_location(app, dir);
            return dir.clone();
        }
    }

    app.path()
        .app_config_dir()
        .expect("could not resolve app config dir")
}

pub fn config_path(app: &AppHandle) -> PathBuf {
    config_dir(app).join("config.toml")
}

pub fn themes_dir(app: &AppHandle) -> PathBuf {
    config_dir(app).join("themes")
}

pub fn load(app: &AppHandle) -> LumaConfig {
    let path = config_path(app);

    match fs::read_to_string(&path) {
        Ok(text) => match toml::from_str(&text) {
            Ok(cfg) => {
                crate::logging::info(app, format!("config loaded from {path:?}"));
                cfg
            }
            Err(err) => {
                crate::logging::error(
                    app,
                    format!("failed to parse {path:?} ({err}) - using defaults"),
                );
                LumaConfig::default()
            }
        },
        Err(_) => {
            crate::logging::info(app, format!("no config at {path:?} yet - writing defaults"));
            let cfg = LumaConfig::default();
            if let Err(err) = save(app, &cfg) {
                crate::logging::error(app, format!("failed to write default config: {err}"));
            }
            cfg
        }
    }
}

pub fn save(app: &AppHandle, cfg: &LumaConfig) -> Result<(), String> {
    let path = config_path(app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = toml::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())
}

const BUILTIN_THEMES: &[(&str, &str, &str)] = &[
    (
        "luma-default",
        include_str!("../resources/themes/luma-default/theme.css"),
        include_str!("../resources/themes/luma-default/theme.json"),
    ),
    (
        "luma-material-blue",
        include_str!("../resources/themes/luma-material-blue/theme.css"),
        include_str!("../resources/themes/luma-material-blue/theme.json"),
    ),
    (
        "luma-pink",
        include_str!("../resources/themes/luma-pink/theme.css"),
        include_str!("../resources/themes/luma-pink/theme.json"),
    ),
    (
        "luma-emerald",
        include_str!("../resources/themes/luma-emerald/theme.css"),
        include_str!("../resources/themes/luma-emerald/theme.json"),
    ),
    (
        "luma-amber",
        include_str!("../resources/themes/luma-amber/theme.css"),
        include_str!("../resources/themes/luma-amber/theme.json"),
    ),
];

pub fn ensure_themes_dir(app: &AppHandle) {
    for (id, css, json) in BUILTIN_THEMES {
        let dest = themes_dir(app).join(id);

        if let Err(err) = fs::create_dir_all(&dest) {
            crate::logging::error(app, format!("failed to seed built-in theme {id}: {err}"));
            continue;
        }
        if let Err(err) = fs::write(dest.join("theme.css"), css) {
            crate::logging::error(app, format!("failed to seed {id} theme.css: {err}"));
        }
        if let Err(err) = fs::write(dest.join("theme.json"), json) {
            crate::logging::error(app, format!("failed to seed {id} theme.json: {err}"));
        }
    }
}
