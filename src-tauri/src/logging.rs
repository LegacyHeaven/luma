use serde::Serialize;
use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
use tauri::{AppHandle, Emitter, Manager};

const MAX_LOG_ENTRIES: usize = 2000;
const MAX_LOG_FILE_BYTES: u64 = 5 * 1024 * 1024;
const LOG_FILE_NAME: &str = "luma.log";
const LOG_FILE_OLD_NAME: &str = "luma.log.old";

pub const BUILD_SHA: &str = concat!(env!("LUMA_BUILD_SHA"), env!("LUMA_BUILD_DIRTY"));

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub ts_ms: u64,
    pub level: String,

    pub source: String,
    pub message: String,
}

struct LogFile {
    dir: PathBuf,
    handle: Option<File>,
}

impl LogFile {
    fn new(dir: PathBuf) -> Self {
        Self { dir, handle: None }
    }

    fn path(&self) -> PathBuf {
        self.dir.join(LOG_FILE_NAME)
    }

    fn old_path(&self) -> PathBuf {
        self.dir.join(LOG_FILE_OLD_NAME)
    }

    fn ensure_open(&mut self) -> std::io::Result<&mut File> {
        if self.handle.is_none() {
            fs::create_dir_all(&self.dir)?;
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.path())?;
            self.handle = Some(file);
        }
        Ok(self.handle.as_mut().unwrap())
    }

    fn rotate_if_needed(&mut self) {
        let path = self.path();
        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size < MAX_LOG_FILE_BYTES {
            return;
        }
        self.handle = None;
        let _ = fs::rename(&path, self.old_path());
    }

    fn write_line(&mut self, line: &str) {
        self.rotate_if_needed();
        if let Ok(file) = self.ensure_open() {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
        }
    }

    fn close(&mut self) {
        self.handle = None;
    }
}

pub struct AppLog {
    entries: Mutex<VecDeque<LogEntry>>,
    enabled: AtomicBool,
    file: Mutex<Option<LogFile>>,
}

impl AppLog {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(MAX_LOG_ENTRIES)),
            enabled: AtomicBool::new(false),
            file: Mutex::new(None),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, app: &AppHandle, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);

        let mut file = self.file.lock().unwrap();
        if enabled {
            if file.is_none() {
                *file = Some(LogFile::new(crate::config::config_dir(app).join("logs")));
            }
        } else if let Some(f) = file.as_mut() {
            f.close();
        }
    }

    pub fn log_file_path(&self, app: &AppHandle) -> PathBuf {
        crate::config::config_dir(app)
            .join("logs")
            .join(LOG_FILE_NAME)
    }

    pub fn snapshot(&self) -> Vec<LogEntry> {
        self.entries.lock().unwrap().iter().cloned().collect()
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    fn push(&self, entry: LogEntry) {
        if !self.is_enabled() {
            return;
        }

        {
            let mut entries = self.entries.lock().unwrap();
            if entries.len() >= MAX_LOG_ENTRIES {
                entries.pop_front();
            }
            entries.push_back(entry.clone());
        }

        let mut file = self.file.lock().unwrap();
        if let Some(f) = file.as_mut() {
            let line = format!(
                "[{}] [{:>5}] [{}] {}",
                format_ts(entry.ts_ms),
                entry.level.to_uppercase(),
                entry.source,
                entry.message
            );
            f.write_line(&line);
        }
    }
}

fn format_ts(ts_ms: u64) -> String {
    let secs = ts_ms / 1000;
    let millis = ts_ms % 1000;
    let days = secs / 86400;
    let rem = secs % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    let mut civil_days = days as i64 + 719468;
    let era = if civil_days >= 0 {
        civil_days
    } else {
        civil_days - 146096
    } / 146097;
    civil_days -= era * 146097;
    let yoe = civil_days;
    let doe = yoe;
    let yoe4 = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe4 + era * 400;
    let doy = doe - (365 * yoe4 + yoe4 / 4 - yoe4 / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    format!("{year:04}-{month:02}-{d:02}T{h:02}:{m:02}:{s:02}.{millis:03}Z")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

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
        if state.is_enabled() {
            let _ = app.emit("luma://log", &entry);
        }
    }
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

pub fn debug(app: &AppHandle, message: impl Into<String>) {
    log(app, "debug", "rust", message);
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub build_sha: String,
    pub app_version: String,
    pub pid: u32,
    pub process_rss_bytes: u64,
    pub process_uptime_secs: u64,

    pub process_tree_rss_bytes: u64,
    pub process_tree: Vec<ProcessInfo>,
    pub system_total_mem_bytes: u64,
    pub system_used_mem_bytes: u64,
    pub os: String,
    pub os_version: String,
    pub config_dir: String,
    pub log_file_path: String,
    pub log_file_bytes: u64,
    pub advanced_logging: bool,
    pub shortcut: String,
    pub browser_mode: String,
    pub theme: String,

    pub windows: Vec<WindowInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub rss_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowInfo {
    pub label: String,
    pub visible: bool,
}

fn collect_process_tree(sys: &System, root_pid: Pid) -> Vec<ProcessInfo> {
    let mut children_of: std::collections::HashMap<Pid, Vec<Pid>> =
        std::collections::HashMap::new();
    for (pid, proc_) in sys.processes() {
        if let Some(parent) = proc_.parent() {
            children_of.entry(parent).or_default().push(*pid);
        }
    }

    let mut result = Vec::new();
    let mut stack = vec![root_pid];
    let mut seen = std::collections::HashSet::new();

    while let Some(pid) = stack.pop() {
        if !seen.insert(pid) {
            continue;
        }
        if let Some(proc_) = sys.process(pid) {
            result.push(ProcessInfo {
                pid: pid.as_u32(),
                name: proc_.name().to_string_lossy().to_string(),
                rss_bytes: proc_.memory(),
            });
        }
        if let Some(kids) = children_of.get(&pid) {
            stack.extend(kids.iter().copied());
        }
    }

    result
}

#[tauri::command]
pub fn get_system_info(
    app: AppHandle,
    state: tauri::State<crate::commands::AppState>,
    log_state: tauri::State<AppLog>,
) -> SystemInfo {
    let pid = std::process::id();

    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_memory();

    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let (rss, uptime) = sys
        .process(Pid::from_u32(pid))
        .map(|p| (p.memory(), p.run_time()))
        .unwrap_or((0, 0));

    let process_tree = collect_process_tree(&sys, Pid::from_u32(pid));
    let process_tree_rss_bytes = process_tree.iter().map(|p| p.rss_bytes).sum();

    let cfg = state.config.lock().unwrap().clone();

    let windows = app
        .webview_windows()
        .iter()
        .map(|(label, w)| WindowInfo {
            label: label.clone(),
            visible: w.is_visible().unwrap_or(false),
        })
        .collect();

    let log_file_path = log_state.log_file_path(&app);
    let log_file_bytes = fs::metadata(&log_file_path).map(|m| m.len()).unwrap_or(0);

    SystemInfo {
        build_sha: BUILD_SHA.to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        pid,
        process_rss_bytes: rss,
        process_uptime_secs: uptime,
        process_tree_rss_bytes,
        process_tree,
        system_total_mem_bytes: sys.total_memory(),
        system_used_mem_bytes: sys.used_memory(),
        os: System::name().unwrap_or_else(|| "unknown".into()),
        os_version: System::os_version().unwrap_or_else(|| "unknown".into()),
        config_dir: crate::config::config_dir(&app)
            .to_string_lossy()
            .to_string(),
        log_file_path: log_file_path.to_string_lossy().to_string(),
        log_file_bytes,
        advanced_logging: log_state.is_enabled(),
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

#[tauri::command]
pub fn log_client_event(app: AppHandle, level: String, message: String) {
    log(&app, &level, "js", message);
}

#[tauri::command]
pub fn reveal_log_file(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = crate::config::config_dir(&app).join("logs");
    let _ = fs::create_dir_all(&dir);
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}
