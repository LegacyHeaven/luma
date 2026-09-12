use crate::{config, themes::ThemeInfo};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use tauri::AppHandle;

const MARKETPLACE_INDEX_URL: &str =
    "https://raw.githubusercontent.com/LegacyHeaven/luma/main/marketplace/index.json";

const MAX_THEME_FILE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    pub id: String,
    pub name: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub css_url: String,
    #[serde(default)]
    pub json_url: Option<String>,
    #[serde(default)]
    pub colors: Vec<String>,
}

#[tauri::command]
pub fn fetch_marketplace_index() -> Result<Vec<MarketplaceEntry>, String> {
    let body = ureq::get(MARKETPLACE_INDEX_URL)
        .call()
        .map_err(|e| format!("request to the marketplace index failed: {e}"))?
        .into_string()
        .map_err(|e| format!("reading the marketplace index failed: {e}"))?;
    serde_json::from_str(&body).map_err(|e| format!("marketplace index didn't parse: {e}"))
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for c in input.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "custom-theme".to_string()
    } else {
        trimmed
    }
}

fn fetch_text(url: &str) -> Result<String, String> {
    if !url.starts_with("https://") {
        return Err("only https:// URLs are allowed".to_string());
    }
    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("request to {url} failed: {e}"))?;
    let mut body = String::new();
    resp.into_reader()
        .take(MAX_THEME_FILE_BYTES + 1)
        .read_to_string(&mut body)
        .map_err(|e| format!("reading {url} failed: {e}"))?;
    if body.len() as u64 > MAX_THEME_FILE_BYTES {
        return Err(format!("{url} is over 512KB - refusing to install"));
    }
    if body.trim().is_empty() {
        return Err(format!("{url} came back empty"));
    }
    Ok(body)
}

/// Fetches a theme's CSS (and optional theme.json) from arbitrary URLs and
/// installs it into the user's local themes folder, where the existing
/// `list_themes`/theme picker already picks up any folder with a
/// `theme.json` - no changes needed there. Used both for one-click installs
/// from the marketplace manifest and for the "add a theme by URL" box.
#[tauri::command]
pub fn install_theme_from_url(
    app: AppHandle,
    css_url: String,
    json_url: Option<String>,
    id_hint: Option<String>,
    name_hint: Option<String>,
    author_hint: Option<String>,
) -> Result<ThemeInfo, String> {
    let css = fetch_text(&css_url)?;
    if !css.contains('{') || !css.contains('}') {
        return Err("that URL doesn't look like a CSS file".to_string());
    }

    let json = match &json_url {
        Some(u) => fetch_text(u)?,
        None => {
            let id = slugify(
                id_hint
                    .as_deref()
                    .or(name_hint.as_deref())
                    .unwrap_or("custom-theme"),
            );
            let name = name_hint.clone().unwrap_or_else(|| id.clone());
            let author = author_hint
                .clone()
                .unwrap_or_else(|| "community".to_string());
            serde_json::json!({
                "id": id,
                "name": name,
                "author": author,
                "type": "dark",
                "css": "theme.css",
            })
            .to_string()
        }
    };

    let manifest: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| format!("theme.json didn't parse: {e}"))?;
    let folder_id = slugify(
        manifest
            .get("id")
            .and_then(|v| v.as_str())
            .or(id_hint.as_deref())
            .unwrap_or("custom-theme"),
    );

    let dest = config::themes_dir(&app).join(&folder_id);
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    fs::write(dest.join("theme.css"), &css).map_err(|e| e.to_string())?;
    fs::write(dest.join("theme.json"), &json).map_err(|e| e.to_string())?;

    crate::logging::info(
        &app,
        format!("installed marketplace theme '{folder_id}' from {css_url}"),
    );

    crate::themes::find(&app, &folder_id)
        .ok_or_else(|| "theme installed but couldn't be read back".to_string())
}
