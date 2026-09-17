use crate::config;
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::AppHandle;

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
