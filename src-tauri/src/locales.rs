use crate::config;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;

pub const DEFAULT_LOCALE_ID: &str = "en";

const BUILTIN_LOCALES: &[(&str, &str, &str)] = &[
    (
        "en",
        "English",
        include_str!("../resources/locales/en.json"),
    ),
    (
        "nl",
        "Nederlands",
        include_str!("../resources/locales/nl.json"),
    ),
];

#[derive(Debug, Clone, Serialize)]
pub struct LocaleInfo {
    pub id: String,
    pub name: String,
}

pub fn locales_dir(app: &AppHandle) -> PathBuf {
    config::config_dir(app).join("locales")
}

pub fn ensure_locales_dir(app: &AppHandle) {
    let dir = locales_dir(app);
    if let Err(err) = fs::create_dir_all(&dir) {
        crate::logging::error(app, format!("failed to create locales dir: {err}"));
        return;
    }
    for (id, _name, json) in BUILTIN_LOCALES {
        let dest = dir.join(format!("{id}.json"));
        if let Err(err) = fs::write(&dest, json) {
            crate::logging::error(app, format!("failed to seed {id}.json locale: {err}"));
        }
    }
}

fn parse_map(text: &str) -> Option<BTreeMap<String, String>> {
    serde_json::from_str(text).ok()
}

fn builtin_json(id: &str) -> Option<&'static str> {
    BUILTIN_LOCALES
        .iter()
        .find(|(builtin_id, _, _)| *builtin_id == id)
        .map(|(_, _, json)| *json)
}

pub fn strings_for(app: &AppHandle, locale_id: &str) -> BTreeMap<String, String> {
    let mut merged = parse_map(builtin_json(DEFAULT_LOCALE_ID).unwrap_or("{}")).unwrap_or_default();

    if locale_id == DEFAULT_LOCALE_ID {
        merged.remove("_display_name");
        return merged;
    }

    let disk_path = locales_dir(app).join(format!("{locale_id}.json"));
    let text = fs::read_to_string(&disk_path)
        .ok()
        .or_else(|| builtin_json(locale_id).map(str::to_string));

    match text {
        Some(text) => match parse_map(&text) {
            Some(overrides) => {
                for (key, value) in overrides {
                    if key == "_display_name" {
                        continue;
                    }
                    merged.insert(key, value);
                }
            }
            None => {
                crate::logging::error(
                    app,
                    format!("locale '{locale_id}' is not valid JSON - showing English instead"),
                );
            }
        },
        None => {
            crate::logging::warn(
                app,
                format!("locale '{locale_id}' not found - showing English instead"),
            );
        }
    }

    merged.remove("_display_name");
    merged
}

pub fn list_locales(app: &AppHandle) -> Vec<LocaleInfo> {
    ensure_locales_dir(app);

    let mut locales: Vec<LocaleInfo> = BUILTIN_LOCALES
        .iter()
        .map(|(id, name, _)| LocaleInfo {
            id: id.to_string(),
            name: name.to_string(),
        })
        .collect();

    let dir = locales_dir(app);
    let Ok(entries) = fs::read_dir(&dir) else {
        return locales;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else {
            continue;
        };
        if locales.iter().any(|l| l.id == id) {
            continue;
        }

        let display_name = fs::read_to_string(&path)
            .ok()
            .and_then(|text| parse_map(&text))
            .and_then(|map| map.get("_display_name").cloned())
            .unwrap_or_else(|| id.clone());

        locales.push(LocaleInfo {
            id,
            name: display_name,
        });
    }

    locales
}
