use crate::config;
use serde::Serialize;
use std::fs;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize)]
pub struct ThemeInfo {
    pub id: String,
    pub name: String,
    pub author: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// Absolute filesystem path to the theme's CSS file.
    pub css_path: String,
}

/// Lists every theme found under `<config dir>/themes/*/theme.json`.
/// This is the user-writable "skins folder" - dropping a new theme folder
/// there (with a theme.json + theme.css) is all it takes to make it
/// selectable, no restart required.
pub fn list_themes(app: &AppHandle) -> Vec<ThemeInfo> {
    config::ensure_themes_dir(app);
    let dir = config::themes_dir(app);

    let mut themes = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else {
        return themes;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("theme.json");
        let Ok(manifest_text) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&manifest_text) else {
            continue;
        };

        let folder_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let css_file = manifest
            .get("css")
            .and_then(|v| v.as_str())
            .unwrap_or("theme.css");

        themes.push(ThemeInfo {
            id: manifest
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(&folder_name)
                .to_string(),
            name: manifest
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&folder_name)
                .to_string(),
            author: manifest
                .get("author")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            kind: manifest
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("dark")
                .to_string(),
            css_path: path.join(css_file).to_string_lossy().to_string(),
        });
    }

    themes
}

pub fn find(app: &AppHandle, theme_id: &str) -> Option<ThemeInfo> {
    list_themes(app).into_iter().find(|t| t.id == theme_id)
}

pub fn css_for(app: &AppHandle, theme_id: &str) -> Option<String> {
    find(app, theme_id).and_then(|t| fs::read_to_string(t.css_path).ok())
}
