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

    pub locale: String,

    pub frecency_ranking: bool,
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
            locale: "en".into(),
            frecency_ranking: false,
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
            theme: DEFAULT_THEME_ID.into(),
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
            main_window_size: "roomy".into(),
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
            enabled_builtin_engines: vec!["Google".into(), "Local".into(), "Open".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PluginsConfig {
    pub enabled_plugins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FrecencyConfig {
    pub bang_usage: std::collections::HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LumaConfig {
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub window: WindowConfig,
    pub search: SearchConfig,
    pub plugins: PluginsConfig,
    pub frecency: FrecencyConfig,
}

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

fn rename_mypc_to_local(app: &AppHandle, mut cfg: LumaConfig) -> LumaConfig {
    let mut changed = false;
    for name in cfg.search.enabled_builtin_engines.iter_mut() {
        if name == "MyPC" {
            *name = "Local".into();
            changed = true;
        }
    }
    if cfg.general.default_engine == "MyPC" {
        cfg.general.default_engine = "Local".into();
        changed = true;
    }
    if changed {
        crate::logging::info(app, "config: migrated old 'MyPC' engine name to 'Local'");
        if let Err(err) = save(app, &cfg) {
            crate::logging::error(app, format!("failed to save MyPC->Local migration: {err}"));
        }
    }
    cfg
}

pub fn load(app: &AppHandle) -> LumaConfig {
    let path = config_path(app);

    match fs::read_to_string(&path) {
        Ok(text) => match toml::from_str(&text) {
            Ok(cfg) => {
                crate::logging::info(app, format!("config loaded from {path:?}"));
                rename_mypc_to_local(app, cfg)
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

pub const DEFAULT_THEME_ID: &str = "luma-default";

const BUILTIN_THEMES: &[(&str, &str, &str)] = &[(
    DEFAULT_THEME_ID,
    include_str!("../resources/themes/luma-default/theme.css"),
    include_str!("../resources/themes/luma-default/theme.json"),
)];

pub fn is_builtin_theme(theme_id: &str) -> bool {
    BUILTIN_THEMES.iter().any(|(id, _, _)| *id == theme_id)
}

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

pub fn plugins_dir(app: &AppHandle) -> PathBuf {
    config_dir(app).join("plugins")
}

pub fn backups_dir(app: &AppHandle) -> PathBuf {
    config_dir(app).join("backups")
}

pub fn desktop_dir(app: &AppHandle) -> PathBuf {
    app.path().desktop_dir().unwrap_or_else(|_| config_dir(app))
}

pub fn ensure_plugins_dir(app: &AppHandle) {
    if let Err(err) = fs::create_dir_all(plugins_dir(app)) {
        crate::logging::error(app, format!("failed to create plugins dir: {err}"));
    }
}
