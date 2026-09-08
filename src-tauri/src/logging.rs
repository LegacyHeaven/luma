//! A small in-memory log the Settings page's debug console (Shift+L) reads
//! from, plus the `get_system_info` command it uses for the RAM/PID/uptime
//! panel.
//!
//! This exists because release builds have
//! `windows_subsystem = "windows"` (see main.rs) - there's no console
//! attached, so a plain `eprintln!` is invisible to anyone who isn't
//! launching Luma from a terminal themselves. Routing every notable event
//! through `log()` here means it's still `eprintln!`'d for that case, but
//! *also* kept in a ring buffer and pushed live to any window listening
//! for `luma://log`, so it's visible from inside the app itself.

use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
use tauri::{AppHandle, Emitter, Manager};

const MAX_LOG_ENTRIES: usize = 1000;

/// Git commit this binary was built from (see build.rs). "unknown" for a
/// build done outside a git checkout (e.g. from a source tarball).
pub const BUILD_SHA: &str = concat!(env!("LUMA_BUILD_SHA"), env!("LUMA_BUILD_DIRTY"));

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub ts_ms: u64,
    pub level: String,
    /// "rust" for anything logged from this side, "js" for events the
    /// frontend reported back via the `log_client_event` command.
    pub source: String,
    pub message: String,
}

pub struct AppLog {
    entries: Mutex<VecDeque<LogEntry>>,
}

impl AppLog {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(MAX_LOG_ENTRIES)),
        }
    }

    pub fn snapshot(&self) -> Vec<LogEntry> {
        self.entries.lock().unwrap().iter().cloned().collect()
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    fn push(&self, entry: LogEntry) {
        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= MAX_LOG_ENTRIES {
            entries.pop_front();
        }
        entries.push_back(entry);
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Records one log line: prints it (visible if Luma was launched from a
/// terminal), stores it in the ring buffer, and emits it live so an open
/// debug console updates immediately without polling.
pub fn log(app: &AppHandle, level: &str, source: &str, message: impl Into<String>) {
    let entry = LogEntry {
        ts_ms: now_ms(),
        level: level.to_string(),
        source: source.to_string(),
        message: message.into(),
    };

    eprintln!("luma[{}][{}]: {}", entry.level, entry.source, entry.message);

    if let Some(state) = app.try_state::<AppLog>() {
        state.push(entry.clone());
    }

    let _ = app.emit("luma://log", &entry);
}

pub fn info(app: &AppHandle, message: impl Into<String>) {
    log(app, "info", "rust", message);
}

pub fn warn(app: &AppHandle, message: impl Into<String>) {
    log(app, "warn", "rust", message);
}

pub fn error(app: &AppHandle, message: impl Into<String>) {
    log(app, "error", "rust", message);
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub build_sha: String,
    pub app_version: String,
    pub pid: u32,
    pub process_rss_bytes: u64,
    pub process_uptime_secs: u64,
    pub system_total_mem_bytes: u64,
    pub system_used_mem_bytes: u64,
    pub os: String,
    pub os_version: String,
    pub config_dir: String,
    pub shortcut: String,
    pub browser_mode: String,
    pub theme: String,
    /// Label + visible state for every currently-existing window (main /
    /// spotlight / browser) - lets you see at a glance whether e.g. a
    /// second spotlight window got created, or the built-in browser window
    /// never opened at all.
    pub windows: Vec<WindowInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowInfo {
    pub label: String,
    pub visible: bool,
}

/// Gathers a snapshot of "what is Luma actually doing right now" - process
/// memory, uptime, the live config, and which windows exist - for the
/// debug console's system-info panel.
#[tauri::command]
pub fn get_system_info(app: AppHandle, state: tauri::State<crate::commands::AppState>) -> SystemInfo {
    let pid = std::process::id();

    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_memory();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), true);

    let (rss, uptime) = sys
        .process(Pid::from_u32(pid))
        .map(|p| (p.memory(), p.run_time()))
        .unwrap_or((0, 0));

    let cfg = state.config.lock().unwrap().clone();

    let windows = app
        .webview_windows()
        .iter()
        .map(|(label, w)| WindowInfo {
            label: label.clone(),
            visible: w.is_visible().unwrap_or(false),
        })
        .collect();

    SystemInfo {
        build_sha: BUILD_SHA.to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        pid,
        process_rss_bytes: rss,
        process_uptime_secs: uptime,
        system_total_mem_bytes: sys.total_memory(),
        system_used_mem_bytes: sys.used_memory(),
        os: System::name().unwrap_or_else(|| "unknown".into()),
        os_version: System::os_version().unwrap_or_else(|| "unknown".into()),
        config_dir: crate::config::config_dir(&app).to_string_lossy().to_string(),
        shortcut: cfg.general.shortcut,
        browser_mode: cfg.general.browser_mode,
        theme: cfg.appearance.theme,
        windows,
    }
}

#[tauri::command]
pub fn get_debug_log(state: tauri::State<AppLog>) -> Vec<LogEntry> {
    state.snapshot()
}

#[tauri::command]
pub fn clear_debug_log(state: tauri::State<AppLog>) {
    state.clear();
}

/// Lets the frontend push its own events (e.g. "search submitted", or an
/// uncaught JS error) into the same unified log the Rust side writes to,
/// so the debug console shows one merged timeline regardless of which
/// window (main/spotlight/settings) something happened in.
#[tauri::command]
pub fn log_client_event(app: AppHandle, level: String, message: String) {
    log(&app, &level, "js", message);
}
