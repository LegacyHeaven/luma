use serde::{Deserialize, Serialize};
use std::fs;
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
