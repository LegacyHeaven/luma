use crate::logging;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use tauri::{AppHandle, Emitter};

const RELEASE_BASE_URL: &str = "https://github.com/LegacyHeaven/luma/releases/latest/download";

#[derive(Debug, Clone, Deserialize)]
struct AssetEntry {
    file: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Manifest {
    commit: String,
    built_at: String,
    assets: HashMap<String, AssetEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateStatus {
    pub available: bool,
    pub current_commit: String,
    pub latest_commit: String,
    pub built_at: String,
    pub checked_ok: bool,
    pub error: Option<String>,
}

fn own_commit() -> String {
    logging::BUILD_SHA.trim_end_matches("-dirty").to_string()
}

fn platform_key() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("linux-x64"),
        ("macos", "aarch64") => Some("macos-arm64"),
        ("macos", "x86_64") => Some("macos-x64"),
        ("windows", "x86_64") => Some("windows-x64"),
        _ => None,
    }
}

fn fetch_manifest() -> Result<Manifest, String> {
    let url = format!("{RELEASE_BASE_URL}/manifest.json");
    let body = ureq::get(&url)
        .call()
        .map_err(|e| format!("request to {url} failed: {e}"))?
        .into_string()
        .map_err(|e| format!("reading manifest.json response failed: {e}"))?;
    serde_json::from_str(&body).map_err(|e| format!("manifest.json didn't parse: {e}"))
}

#[tauri::command]
pub fn check_for_update(app: AppHandle) -> UpdateStatus {
    let current = own_commit();

    match fetch_manifest() {
        Ok(manifest) => {
            let available = manifest.commit != current && current != "unknown";
            logging::info(
                &app,
                format!(
                    "check_for_update: current={current} latest={} available={available}",
                    manifest.commit
                ),
            );
            UpdateStatus {
                available,
                current_commit: current,
                latest_commit: manifest.commit,
                built_at: manifest.built_at,
                checked_ok: true,
                error: None,
            }
        }
        Err(err) => {
            logging::warn(&app, format!("check_for_update failed: {err}"));
            UpdateStatus {
                available: false,
                current_commit: current,
                latest_commit: String::new(),
                built_at: String::new(),
                checked_ok: false,
                error: Some(err),
            }
        }
    }
}

fn emit_progress(app: &AppHandle, stage: &str, detail: impl Into<String>) {
    let _ = app.emit(
        "luma://update-progress",
        serde_json::json!({ "stage": stage, "detail": detail.into() }),
    );
}

#[tauri::command]
pub fn apply_update(app: AppHandle) -> Result<(), String> {
    let key = platform_key().ok_or_else(|| "no update available for this platform".to_string())?;

    emit_progress(&app, "checking", "looking up the latest build");
    let manifest = fetch_manifest()?;
    let asset = manifest
        .assets
        .get(key)
        .ok_or_else(|| format!("manifest.json has no asset for platform '{key}'"))?;

    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let download_url = format!("{RELEASE_BASE_URL}/{}", asset.file);
    let tmp_name = format!(
        "{}.update",
        current_exe
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "luma".into())
    );

    let tmp_path = current_exe.with_file_name(tmp_name);

    logging::info(&app, format!("apply_update: downloading {download_url}"));
    emit_progress(&app, "downloading", format!("fetching {}", asset.file));

    if let Err(err) = download_and_verify(&download_url, &tmp_path, &asset.sha256) {
        let _ = fs::remove_file(&tmp_path);
        logging::error(&app, format!("apply_update: {err}"));
        return Err(err);
    }

    emit_progress(&app, "installing", "replacing the running binary");
    logging::info(&app, "apply_update: checksum OK, installing");

    install_and_relaunch(&app, &current_exe, &tmp_path)
}

fn download_and_verify(url: &str, dest: &Path, expected_sha256: &str) -> Result<(), String> {
    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let mut reader = resp.into_reader();
    let mut out = fs::File::create(dest).map_err(|e| e.to_string())?;

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("download read failed: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
    }
    drop(out);

    let digest = format!("{:x}", hasher.finalize());
    if digest != expected_sha256 {
        return Err(format!(
            "checksum mismatch (downloaded file hashes to {digest}, manifest.json says {expected_sha256}) - not installing"
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn install_and_relaunch(
    app: &AppHandle,
    current_exe: &Path,
    new_path: &Path,
) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(new_path)
        .map_err(|e| e.to_string())?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(new_path, perms).map_err(|e| e.to_string())?;

    fs::rename(new_path, current_exe).map_err(|e| e.to_string())?;

    logging::info(app, "apply_update: installed, relaunching");
    std::process::Command::new(current_exe)
        .spawn()
        .map_err(|e| e.to_string())?;
    std::process::exit(0);
}

#[cfg(target_os = "windows")]
fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut last_err = None;
    for attempt in 0..20 {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(err) => {
                last_err = Some(err);
                if attempt < 19 {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                }
            }
        }
    }
    Err(last_err.expect("loop always sets last_err before running out of attempts"))
}

#[cfg(target_os = "windows")]
fn install_and_relaunch(
    app: &AppHandle,
    current_exe: &Path,
    new_path: &Path,
) -> Result<(), String> {
    let file_name = current_exe
        .file_name()
        .ok_or_else(|| "current executable has no file name".to_string())?
        .to_string_lossy()
        .to_string();
    let old_path = current_exe.with_file_name(format!("{file_name}.old"));
    let _ = fs::remove_file(&old_path);

    rename_with_retry(current_exe, &old_path)
        .map_err(|e| format!("couldn't move the running build aside: {e}"))?;

    if let Err(err) = rename_with_retry(new_path, current_exe) {
        let _ = fs::rename(&old_path, current_exe);
        return Err(format!("couldn't move the new build into place: {err}"));
    }

    logging::info(app, "apply_update: installed, relaunching");
    if let Err(err) = std::process::Command::new(current_exe).spawn() {
        return Err(format!("update installed but relaunch failed: {err}"));
    }

    let _ = fs::remove_file(&old_path);
    std::process::exit(0);
}
