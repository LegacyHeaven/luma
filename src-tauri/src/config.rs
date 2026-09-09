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
    /// Persists whether the Settings page's debug console section (Shift+L
    /// to reveal it) should stay open on future launches. The debug log
    /// itself is always collected regardless of this flag - it only
    /// controls whether the section/console auto-shows.
    pub debug_logging: bool,
    /// Whether the main window silently checks for a new build on launch
    /// and shows the "Update available" banner - see src/updater.rs. The
    /// Settings page's "Check for updates" button works either way.
    pub check_for_updates: bool,
    /// Show a small "LUMA." wordmark above the spotlight bar, mirroring the
    /// main window's brand line (but never the motto - the spotlight stays
    /// a single search line either way).
    pub show_spotlight_branding: bool,
    /// Disables the fade-in/out on the spotlight and the main window's
    /// entrance motion - for anyone who finds it distracting, or a machine
    /// where it's just extra work for no benefit.
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
    /// "top-center" (legacy), "center" (default), or "custom" (a point the
    /// user picked - see spotlight_custom_x/y below).
    pub spotlight_position: String,
    /// Fraction (0.0-1.0) of the primary monitor's width/height where the
    /// user clicked with Settings' "Pick position" overlay - only
    /// meaningful when spotlight_position == "custom". Stored as a
    /// fraction rather than raw pixels so it survives a resolution change
    /// reasonably. See window::position_spotlight / window::open_position_picker.
    pub spotlight_custom_x: Option<f64>,
    pub spotlight_custom_y: Option<f64>,
    /// "compact" | "default" | "roomy" - see window::main_window_dimensions().
    /// Julian's feedback was that the main window felt too big by default,
    /// so "default" here is deliberately smaller than the original 900x640,
    /// and "compact" gives an even smaller option.
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

/// A search engine the user added themselves in Settings, on top of the
/// built-in catalog (see commands::get_engines, which merges the two for
/// the frontend). Deliberately simpler than the built-in `EngineDef`
/// shape in resources/engines.json (no separate `param`/`custom` modes to
/// explain) - just one field to fill in: a URL with `%s` standing in for
/// the search text, e.g. `https://example.com/search?q=%s`. See
/// vendor/engine/bangdeck.js's `template` handling for the other half of
/// this.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomEngine {
    pub name: String,
    /// A URL containing at least one literal `%s`, replaced with the
    /// percent-encoded query at search time.
    pub action: String,
    /// Bang word, without the leading "!" - stored lowercase.
    pub bang: String,
    #[serde(default)]
    pub placeholder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SearchConfig {
    /// User-added engines from Settings' "Search engines" section - see
    /// commands::add_custom_engine/remove_custom_engine.
    pub custom_engines: Vec<CustomEngine>,
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

/// Load config.toml, creating it with defaults if it doesn't exist yet.
/// A config file with a parse error also falls back to defaults rather
/// than crashing the app.
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

/// The built-in themes' files, compiled straight into the binary. Seeding
/// `<config dir>/themes/<id>` from these (rather than from resource files
/// next to the executable) is what lets a bare downloaded `luma` binary
/// work with zero other files alongside it.
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

/// Makes sure every built-in theme folder under `<config dir>/themes/`
/// exists and matches the copy compiled into this binary.
///
/// This always overwrites the built-in themes' files (not just on first
/// run) - they're not a place users are meant to edit in place (the
/// Theming docs tell people to copy the folder first), so re-seeding them
/// on every launch is what makes a Luma update actually change how the
/// app looks instead of a user's on-disk copy silently going stale. Anyone
/// customizing keeps their own theme folder, which this never touches.
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
