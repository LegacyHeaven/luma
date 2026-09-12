// Discord-style first run: Luma ships as a single portable executable (see
// release.yml's own comment - "no installer, just one file per platform").
// That's great for portability but means a fresh download sitting in
// ~/Downloads has no Start Menu entry, no desktop icon, and updates itself
// wherever the user happened to leave it.
//
// The first time a build runs from somewhere that isn't its own install
// folder, this copies itself into a stable per-user app folder, drops a
// desktop shortcut (and Start Menu entry on Windows) next to it, and
// relaunches from there - the same first-run handoff Discord/Slack/etc.
// give you from their tiny installer, without needing an actual installer,
// admin rights, or a new dependency.
//
// This runs before the Tauri app is even built (see main.rs's very first
// line) so a relocation is a clean process handoff: the original process
// never enters the Tauri event loop, the single-instance lock, or the
// tray - it just spawns the installed copy and exits. Every step is
// best-effort: if anything here fails (no write access, disk full,
// cscript missing, whatever), Luma just keeps running from wherever it
// was launched instead of refusing to start. A portable exe that still
// works beats one that got clever and broke.

use std::path::{Path, PathBuf};

pub fn maybe_relocate_and_relaunch() {
    let Some(installed_dir) = install_dir() else {
        return;
    };
    let Ok(current_exe) = std::env::current_exe() else {
        return;
    };

    let target = installed_dir.join(exe_name());

    // Already running from the installed location - including a build the
    // in-app updater has since swapped in place at that same path - so
    // there's nothing to relocate.
    if paths_match(&current_exe, &target) {
        return;
    }

    if let Err(err) = relocate(&current_exe, &installed_dir, &target) {
        eprintln!("luma: first-run install step skipped: {err}");
        return;
    }

    // Best-effort - a failed shortcut is a cosmetic miss, not a reason to
    // stop installing.
    if let Err(err) = create_shortcuts(&target) {
        eprintln!("luma: couldn't create a shortcut (continuing anyway): {err}");
    }

    match std::process::Command::new(&target).spawn() {
        Ok(_) => std::process::exit(0),
        Err(err) => {
            eprintln!(
                "luma: installed to {} but relaunch failed ({err}) - continuing from the original copy",
                target.display()
            );
        }
    }
}

fn paths_match(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    canon(a) == canon(b)
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "luma.exe"
    } else {
        "luma"
    }
}

#[cfg(target_os = "windows")]
fn install_dir() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(base).join("Luma"))
}

#[cfg(target_os = "linux")]
fn install_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("Luma"),
    )
}

// macOS isn't handled here (yet) - a raw Mach-O binary doesn't get the same
// double-click/Dock treatment as a real .app bundle, and building a proper
// bundle (Info.plist, code signing considerations, etc.) is a bigger job
// than a same-day add. Luma keeps running exactly as it does today on
// macOS; only Windows and Linux get the new first-run relocate.
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn install_dir() -> Option<PathBuf> {
    None
}

fn relocate(current_exe: &Path, installed_dir: &Path, target: &Path) -> Result<(), String> {
    std::fs::create_dir_all(installed_dir).map_err(|e| e.to_string())?;

    let tmp_target = installed_dir.join(format!("{}.new", exe_name()));
    std::fs::copy(current_exe, &tmp_target).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&tmp_target).map_err(|e| e.to_string())?;
        let mut perms = meta.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp_target, perms).map_err(|e| e.to_string())?;
    }

    // Rename into place last so a copy that dies partway through never
    // leaves a half-written exe at the path future launches will check.
    std::fs::rename(&tmp_target, target).map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn create_shortcuts(target: &Path) -> Result<(), String> {
    let target_str = target.to_string_lossy().replace('\'', "''");
    let desktop = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|p| p.join("Desktop").join("Luma.lnk"));
    let start_menu = std::env::var_os("APPDATA").map(PathBuf::from).map(|p| {
        p.join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Luma.lnk")
    });

    let mut made_any = false;
    let mut last_err = None;
    for link_path in [desktop, start_menu].into_iter().flatten() {
        match make_windows_shortcut(&link_path, &target_str) {
            Ok(()) => made_any = true,
            Err(err) => last_err = Some(err),
        }
    }

    if made_any {
        Ok(())
    } else {
        Err(last_err.unwrap_or_else(|| "no shortcut location was writable".to_string()))
    }
}

// No extra crate for this - a tiny VBScript run through the `cscript`
// interpreter that ships with every Windows install is the same trick
// countless installers use to write a .lnk without linking COM bindings.
#[cfg(target_os = "windows")]
fn make_windows_shortcut(link_path: &Path, target_str: &str) -> Result<(), String> {
    let Some(parent) = link_path.parent() else {
        return Err("shortcut path has no parent directory".to_string());
    };
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let link_str = link_path.to_string_lossy().replace('\'', "''");
    let vbs = format!(
        "Set oWS = WScript.CreateObject(\"WScript.Shell\")\r\n\
         Set oLink = oWS.CreateShortcut(\"{link_str}\")\r\n\
         oLink.TargetPath = \"{target_str}\"\r\n\
         oLink.IconLocation = \"{target_str}, 0\"\r\n\
         oLink.Save\r\n"
    );

    let tmp_vbs = std::env::temp_dir().join(format!("luma-shortcut-{}.vbs", std::process::id()));
    std::fs::write(&tmp_vbs, vbs).map_err(|e| e.to_string())?;

    let status = std::process::Command::new("cscript")
        .args(["//nologo", &tmp_vbs.to_string_lossy()])
        .status();

    let _ = std::fs::remove_file(&tmp_vbs);

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("cscript exited with {s}")),
        Err(err) => Err(format!("couldn't run cscript: {err}")),
    }
}

// A standard XDG .desktop entry, both registered with the app menu and
// copied to ~/Desktop so most Linux desktop environments show an icon
// there too. Marking it executable is what GNOME/Nautilus and friends
// require before they'll treat a desktop file as launchable rather than
// showing it as plain text.
#[cfg(target_os = "linux")]
fn create_shortcuts(target: &Path) -> Result<(), String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "no HOME set".to_string())?;

    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Luma\n\
         Comment=The open-source spotlight for the desktop\n\
         Exec=\"{}\"\n\
         Terminal=false\n\
         Categories=Utility;\n",
        target.display()
    );

    let apps_dir = home.join(".local").join("share").join("applications");
    std::fs::create_dir_all(&apps_dir).map_err(|e| e.to_string())?;
    let app_entry = apps_dir.join("luma.desktop");
    std::fs::write(&app_entry, &entry).map_err(|e| e.to_string())?;
    mark_executable(&app_entry)?;

    let desktop_dir = home.join("Desktop");
    if desktop_dir.is_dir() {
        let shortcut = desktop_dir.join("Luma.desktop");
        std::fs::write(&shortcut, &entry).map_err(|e| e.to_string())?;
        mark_executable(&shortcut)?;
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn mark_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let mut perms = meta.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).map_err(|e| e.to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn create_shortcuts(_target: &Path) -> Result<(), String> {
    Ok(())
}
