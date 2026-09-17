use crate::config;
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::AppHandle;

const PLUGIN_MARKETPLACE_INDEX_URL: &str =
    "https://raw.githubusercontent.com/LegacyHeaven/luma/main/marketplace/plugins/index.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginBang {
    pub word: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub author: String,
    pub bangs: Vec<PluginBang>,
    pub js_path: String,
    pub is_builtin: bool,
}

pub fn list_plugins(app: &AppHandle) -> Vec<PluginInfo> {
    config::ensure_plugins_dir(app);
    let dir = config::plugins_dir(app);

    let mut plugins = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else {
        return plugins;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("plugin.json");
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

        let entry_file = manifest
            .get("entry")
            .and_then(|v| v.as_str())
            .unwrap_or("plugin.js");

        let id = manifest
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&folder_name)
            .to_string();

        let bangs: Vec<PluginBang> = manifest
            .get("bangs")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|b| {
                        let word = b.get("word")?.as_str()?.to_string();
                        let name = b
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&word)
                            .to_string();
                        Some(PluginBang { word, name })
                    })
                    .collect()
            })
            .unwrap_or_default();

        plugins.push(PluginInfo {
            is_builtin: config::is_builtin_plugin(&id),
            id,
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
            bangs,
            js_path: path.join(entry_file).to_string_lossy().to_string(),
        });
    }

    plugins
}

pub fn find(app: &AppHandle, plugin_id: &str) -> Option<PluginInfo> {
    list_plugins(app).into_iter().find(|p| p.id == plugin_id)
}

pub fn js_for(app: &AppHandle, plugin_id: &str) -> Option<String> {
    find(app, plugin_id).and_then(|p| fs::read_to_string(p.js_path).ok())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketplaceEntry {
    pub id: String,
    pub name: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub js_url: String,
    #[serde(default)]
    pub json_url: Option<String>,
}

#[tauri::command]
pub fn fetch_plugin_marketplace_index() -> Result<Vec<PluginMarketplaceEntry>, String> {
    let body = ureq::get(PLUGIN_MARKETPLACE_INDEX_URL)
        .call()
        .map_err(|e| format!("request to the plugin marketplace index failed: {e}"))?
        .into_string()
        .map_err(|e| format!("reading the plugin marketplace index failed: {e}"))?;
    serde_json::from_str(&body).map_err(|e| format!("plugin marketplace index didn't parse: {e}"))
}

#[tauri::command]
pub fn install_plugin_from_url(
    app: AppHandle,
    js_url: String,
    json_url: Option<String>,
    id_hint: Option<String>,
    name_hint: Option<String>,
    author_hint: Option<String>,
) -> Result<PluginInfo, String> {
    let js = crate::marketplace::fetch_text(&js_url)?;

    let json = match &json_url {
        Some(u) => crate::marketplace::fetch_text(u)?,
        None => {
            let id = crate::marketplace::slugify(
                id_hint
                    .as_deref()
                    .or(name_hint.as_deref())
                    .unwrap_or("custom-plugin"),
            );
            let name = name_hint.clone().unwrap_or_else(|| id.clone());
            let author = author_hint
                .clone()
                .unwrap_or_else(|| "community".to_string());
            serde_json::json!({
                "id": id,
                "name": name,
                "author": author,
                "entry": "plugin.js",
                "bangs": [],
            })
            .to_string()
        }
    };

    let manifest: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| format!("plugin.json didn't parse: {e}"))?;
    let folder_id = crate::marketplace::slugify(
        manifest
            .get("id")
            .and_then(|v| v.as_str())
            .or(id_hint.as_deref())
            .unwrap_or("custom-plugin"),
    );

    let dest = config::plugins_dir(&app).join(&folder_id);
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    fs::write(dest.join("plugin.js"), &js).map_err(|e| e.to_string())?;
    fs::write(dest.join("plugin.json"), &json).map_err(|e| e.to_string())?;

    crate::logging::info(
        &app,
        format!("installed marketplace plugin '{folder_id}' from {js_url}"),
    );

    find(&app, &folder_id).ok_or_else(|| "plugin installed but couldn't be read back".to_string())
}
