use std::path::PathBuf;

fn remove_shortcuts() {
    #[cfg(target_os = "windows")]
    {
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
        for link in [desktop, start_menu].into_iter().flatten() {
            let _ = std::fs::remove_file(link);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let _ = std::fs::remove_file(
                home.join(".local")
                    .join("share")
                    .join("applications")
                    .join("luma.desktop"),
            );
            let _ = std::fs::remove_file(home.join("Desktop").join("Luma.desktop"));
        }
    }
}

#[cfg(target_os = "windows")]
fn install_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join("Luma"))
}

#[cfg(target_os = "linux")]
fn install_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("Luma")
    })
}

#[cfg(target_os = "windows")]
pub fn run() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    remove_shortcuts();

    let Some(dir) = install_dir() else {
        return Err("couldn't determine the install folder".into());
    };
    if !dir.is_dir() {
        std::process::exit(0);
    }

    let dir_str = dir.to_string_lossy().replace('"', "\"\"");
    let cmd = format!("ping -n 3 127.0.0.1 >nul & rmdir /s /q \"{dir_str}\"");
    std::process::Command::new("cmd")
        .args(["/c", &cmd])
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn()
        .map_err(|e| format!("couldn't schedule cleanup: {e}"))?;

    std::process::exit(0);
}

#[cfg(target_os = "linux")]
pub fn run() -> Result<(), String> {
    remove_shortcuts();

    if let Some(dir) = install_dir() {
        let _ = std::fs::remove_dir_all(dir);
    }

    std::process::exit(0);
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn run() -> Result<(), String> {
    Err("LUMA can't remove itself on this platform yet - quit it and drag LUMA from Applications to the Trash.".into())
}
