//! A hand-rolled updater, not Tauri's official updater plugin.
//!
//! Luma ships as a single "no installer, no bundle" executable per
//! platform (`bundle.active: false` in tauri.conf.json, see release.yml) -
//! the official plugin only knows how to replace specific bundle formats
//! (MSI/NSIS/AppImage/.app), not a bare binary someone downloaded and put
//! wherever they wanted, so it doesn't fit this distribution model.
//!
//! Versioning: release.yml can cut a release under any tag name (Julian
//! wanted updates to keep working "to any newer release, not just one
//! version" rather than being pinned to whichever tag happened to exist
//! when a build shipped) so there's no fixed semver to compare here.
//! Instead this fetches GitHub's "latest release" virtual path
//! (`/releases/latest/download/...`), which always resolves to whichever
//! *published* (non-draft, non-prerelease) release is newest - no tag name
//! baked into the client at all - and compares the running build's
//! embedded git commit (build.rs / logging::BUILD_SHA) against the commit
//! recorded in a small `manifest.json` release asset that release.yml
//! publishes alongside the binaries. "The latest release has a different
//! commit than the one I was built from" is the update signal, and it
//! keeps working across any number of future releases with zero client
//! changes - the exact thing a fixed-tag URL couldn't do (see git history:
//! this is what replaced the earlier `1R`-then-`Release` fixed-tag scheme,
//! which needed a client update every time the tag changed).
//!
//! Installing: the new binary is downloaded next to the current one and
//! checksum-verified against manifest.json before anything touches the
//! running executable. macOS/Linux can rename a file over the path of
//! their own already-running executable (the OS keeps the old inode alive
//! for the current process), so that's a direct swap-and-relaunch.
//! Windows can't - the running .exe is locked - so there a small detached
//! helper script waits for this process to exit, moves the new file into
//! place, and relaunches it; see resources/windows-update-helper.ps1.

use crate::logging;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use tauri::{AppHandle, Emitter};

// GitHub's "latest release" virtual path - always the newest *published*
// release's assets, whatever tag it happens to be under. This is what
// makes "any newer release, not just one version" true: release.yml is
// free to use any tag (or none at all) going forward and the updater
// never needs to change again. (A build from before this change has the
// old fixed-tag URL baked in and needs one manual re-download - every
// build from here on updates itself indefinitely.)
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

/// This build's commit, with any "-dirty" suffix stripped - a local dev
/// build off a modified tree should never be compared as if it were an
/// ordinary release build.
fn own_commit() -> String {
    logging::BUILD_SHA.trim_end_matches("-dirty").to_string()
}

/// The key manifest.json uses for this platform - must match what
/// release.yml writes into it.
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

/// Called on main-window launch (if enabled) and from the Settings page's
/// "Check for updates" button. Never fails loudly to the caller - a
/// failed check (offline, GitHub unreachable) just comes back with
/// `checked_ok: false` so the frontend can quietly skip showing anything
/// rather than nagging with an error banner on every offline launch.
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

/// Where the Windows update-helper script (see install_and_relaunch below
/// and resources/windows-update-helper.ps1) leaves a note if it ultimately
/// fails to swap the files. Only relevant on Windows: the process that
/// kicks off `apply_update` there has to exit unconditionally before the
/// swap can happen (the running .exe can't replace itself), so if the
/// swap then fails there is no live Tauri command left to report an error
/// through - this file on disk is the only channel back to the next
/// launch. macOS/Linux do the swap in-process before exiting, so a
/// failure there already returns a normal `Err` from `apply_update` and
/// never needs this.
fn update_failure_marker_path() -> std::path::PathBuf {
    std::env::temp_dir().join("luma-update-failed.txt")
}

/// Called once on startup by the main window (see main.js's boot()) right
/// alongside check_for_update. Reads and clears the marker above, if one
/// is there, so the frontend can tell the user the last automatic update
/// didn't take instead of it just quietly having not happened.
#[tauri::command]
pub fn take_last_update_failure() -> Option<String> {
    let path = update_failure_marker_path();
    let text = fs::read_to_string(&path).ok()?;
    let _ = fs::remove_file(&path);
    Some(text)
}

/// Downloads, verifies, and installs the update for this platform, then
/// relaunches - on success this process exits and never actually returns
/// `Ok`, so the frontend's `invoke("apply_update")` promise is expected to
/// just hang until the window closes rather than resolve normally.
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
    // Downloaded next to the current executable (not into a system temp
    // dir) so the final install step is a same-filesystem rename, never a
    // cross-device copy that could fail partway through.
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

    // Renaming over our own already-running executable is safe on
    // Unix-likes: the process keeps its existing file mapped by inode, so
    // this doesn't crash the process that's doing the renaming.
    fs::rename(new_path, current_exe).map_err(|e| e.to_string())?;

    logging::info(app, "apply_update: installed, relaunching");
    std::process::Command::new(current_exe)
        .spawn()
        .map_err(|e| e.to_string())?;
    std::process::exit(0);
}

#[cfg(target_os = "windows")]
fn install_and_relaunch(
    app: &AppHandle,
    current_exe: &Path,
    new_path: &Path,
) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    // 0x08 = DETACHED_PROCESS, 0x00000200 = CREATE_NEW_PROCESS_GROUP - the
    // helper needs to keep running after this process exits, with no
    // console window flashing up.
    const DETACHED_PROCESS: u32 = 0x00000008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

    let pid = std::process::id();
    let script = include_str!("../resources/windows-update-helper.ps1")
        .replace("__PID__", &pid.to_string())
        .replace("__OLD__", &current_exe.to_string_lossy())
        .replace("__NEW__", &new_path.to_string_lossy())
        .replace(
            "__MARKER__",
            &update_failure_marker_path().to_string_lossy(),
        )
        .replace(
            "__LOG__",
            &std::env::temp_dir()
                .join(format!("luma-update-{pid}.log"))
                .to_string_lossy(),
        );

    let script_path = std::env::temp_dir().join(format!("luma-update-{pid}.ps1"));
    fs::write(&script_path, script).map_err(|e| e.to_string())?;

    logging::info(app, "apply_update: spawning Windows update helper, exiting");
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .map_err(|e| e.to_string())?;

    std::process::exit(0);
}
