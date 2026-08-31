use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// e.g. "Alt+Space" - parsed by tauri-plugin-global-shortcut.
    pub shortcut: String,
    /// "system" (your default OS browser) or "builtin" (Luma's own webview window).
    pub browser_mode: String,
    /// Engine name (matches an entry in engines.json) used when no !bang is typed.
    pub default_engine: String,
    pub start_at_login: bool,
    /// Hide the spotlight window automatically when it loses focus.
    pub close_spotlight_on_blur: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            shortcut: "Alt+Space".into(),
            browser_mode: "system".into(),
            default_engine: "Google".into(),
            start_at_login: false,
            close_spotlight_on_blur: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    /// Theme id - must match a folder name under the themes directory.
    pub theme: String,
    /// Extra CSS appended after the theme's stylesheet.
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
    /// "top-center" or "center"
    pub spotlight_position: String,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            spotlight_width: 640,
            spotlight_position: "top-center".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LumaConfig {
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub window: WindowConfig,
}

pub fn config_dir(app: &AppHandle) -> PathBuf {
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

/// Load config.toml, creating it with defaults if it doesn't exist yet.
/// A config file with a parse error also falls back to defaults rather
/// than crashing the app.
pub fn load(app: &AppHandle) -> LumaConfig {
    let path = config_path(app);

    match fs::read_to_string(&path) {
        Ok(text) => toml::from_str(&text).unwrap_or_else(|err| {
            eprintln!("luma: failed to parse {path:?} ({err}) - using defaults");
            LumaConfig::default()
        }),
        Err(_) => {
            let cfg = LumaConfig::default();
            if let Err(err) = save(app, &cfg) {
                eprintln!("luma: failed to write default config: {err}");
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

/// The default theme's files, compiled straight into the binary. Seeding
/// `<config dir>/themes/luma-default` from these (rather than from a
/// resource file next to the executable) is what lets a bare downloaded
/// `luma` binary work with zero other files alongside it.
const DEFAULT_THEME_CSS: &str = include_str!("../resources/themes/luma-default/theme.css");
const DEFAULT_THEME_JSON: &str = include_str!("../resources/themes/luma-default/theme.json");

/// Makes sure `<config dir>/themes/luma-default` exists and matches the
/// copy of the theme compiled into this binary.
///
/// This always overwrites `luma-default`'s files (not just on first run) -
/// it's the built-in theme, not a place users are meant to edit in place
/// (the Theming docs tell people to copy the folder first), so re-seeding
/// it on every launch is what makes a Luma update actually change how the
/// app looks instead of a user's on-disk copy silently going stale. Anyone
/// customizing keeps their own theme folder, which this never touches.
pub fn ensure_themes_dir(app: &AppHandle) {
    let dest = themes_dir(app).join("luma-default");

    if let Err(err) = fs::create_dir_all(&dest) {
        eprintln!("luma: failed to seed default theme: {err}");
        return;
    }
    if let Err(err) = fs::write(dest.join("theme.css"), DEFAULT_THEME_CSS) {
        eprintln!("luma: failed to seed default theme: {err}");
    }
    if let Err(err) = fs::write(dest.join("theme.json"), DEFAULT_THEME_JSON) {
        eprintln!("luma: failed to seed default theme: {err}");
    }
}
