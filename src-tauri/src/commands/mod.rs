//! Commands callable from the frontend. Every argument is validated here; the
//! frontend can never supply a file path to open or run.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::DialogExt;

use crate::csv;
use crate::models::*;
use crate::processes;
use crate::settings::Settings;
use crate::state::{lock, AppState};
use crate::tray;
use crate::util::REPOSITORY_URL;

type S<'a> = State<'a, Arc<AppState>>;
type CmdResult<T> = Result<T, String>;

const MAX_EXPORT_BYTES: usize = 100 * 1024 * 1024;

#[tauri::command]
pub fn get_app_info(state: S<'_>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        repository: REPOSITORY_URL.to_string(),
        platform_supported: cfg!(windows),
        portable: state.portable,
    }
}

#[tauri::command]
pub fn get_snapshot(state: S<'_>) -> Snapshot {
    let mut snap = state.snapshot.read().unwrap_or_else(|e| e.into_inner()).clone();
    snap.paused = state.paused.load(Ordering::SeqCst);
    snap.platform_supported = cfg!(windows);
    snap.interval_ms = state.settings.get().refresh_interval_ms;
    snap.unread_alerts = state.unread_alerts.load(Ordering::SeqCst);
    snap
}

#[tauri::command]
pub fn set_paused(state: S<'_>, paused: bool) {
    tray::set_paused(&state, paused);
}

#[tauri::command]
pub fn refresh_now(state: S<'_>) {
    state.wake();
}

/// Live lookup for the details panel. Never fails: a process that exited or
/// cannot be opened is described in the result instead.
#[tauri::command]
pub async fn get_process_details(pid: u32) -> ProcessDetails {
    if pid == 0 {
        return ProcessDetails {
            pid,
            name: None,
            path: None,
            start_time: None,
            running: false,
            accessible: false,
            note: Some("Windows did not report an owning process for this connection (PID 0). This is normal for connections that are closing (TIME_WAIT).".into()),
        };
    }
    let q = processes::query(pid);
    let name = processes::process_names().get(&pid).cloned();
    let note = if !q.running {
        Some("This process is no longer running.".to_string())
    } else if !q.accessible {
        Some("Windows does not allow PortWatch to inspect this process (it is likely protected or runs with higher privileges). Run PortWatch as administrator to see more; this is not required for normal use.".to_string())
    } else if q.path.is_none() {
        Some("Windows did not provide an executable path for this process.".to_string())
    } else {
        None
    };
    ProcessDetails {
        pid,
        name: if q.running { name } else { None },
        path: q.path,
        start_time: q.start_time,
        running: q.running,
        accessible: q.accessible,
        note,
    }
}

/// Opens Explorer on the executable. The path is looked up again from the PID;
/// nothing path-like is accepted from the frontend.
#[tauri::command]
pub async fn reveal_process_folder(pid: u32) -> CmdResult<()> {
    let q = processes::query(pid);
    let path = q
        .path
        .ok_or_else(|| "The executable path is unavailable for this process.".to_string())?;
    processes::reveal_in_explorer(&path)
}

#[tauri::command]
pub fn get_recent_events(state: S<'_>, limit: Option<usize>) -> Vec<HistoryEvent> {
    let ring = lock(&state.recent_events);
    let limit = limit.unwrap_or(500).min(5000);
    ring.iter().rev().take(limit).cloned().collect()
}

// ---- history ----------------------------------------------------------

#[tauri::command]
pub async fn query_history(state: S<'_>, filter: HistoryFilter) -> CmdResult<HistoryPage> {
    state.db.query_events(&filter)
}

#[tauri::command]
pub async fn list_history_processes(state: S<'_>) -> CmdResult<Vec<String>> {
    state.db.distinct_processes()
}

#[tauri::command]
pub async fn clear_history(state: S<'_>) -> CmdResult<()> {
    state.db.clear_events()
}

fn safe_file_name(name: &str, fallback: &str) -> String {
    let ok = !name.is_empty()
        && name.len() <= 80
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if ok {
        name.to_string()
    } else {
        fallback.to_string()
    }
}

/// Shows the native "Save as" dialog and writes the CSV where the user chooses.
/// Returns the chosen path, or `None` if the dialog was cancelled.
fn save_csv(app: &AppHandle, default_name: &str, contents: &str) -> CmdResult<Option<String>> {
    if contents.len() > MAX_EXPORT_BYTES {
        return Err("The export is too large to save.".into());
    }
    let name = safe_file_name(default_name, "portwatch-export.csv");
    let Some(file) = app
        .dialog()
        .file()
        .set_file_name(&name)
        .add_filter("CSV file", &["csv"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|e| e.to_string())?;
    // UTF-8 BOM so Excel detects the encoding of non-ASCII process names.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(contents.as_bytes());
    std::fs::write(&path, bytes).map_err(|e| format!("Could not save the file: {e}"))?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
pub async fn save_export(
    app: AppHandle,
    default_name: String,
    contents: String,
) -> CmdResult<Option<String>> {
    save_csv(&app, &default_name, &contents)
}

#[tauri::command]
pub async fn export_history_csv(
    app: AppHandle,
    state: S<'_>,
    filter: HistoryFilter,
) -> CmdResult<Option<String>> {
    let mut f = filter;
    f.limit = Some(1_000_000);
    f.offset = Some(0);
    let page = state.db.query_events(&f)?;
    let csv_text = csv::history_csv(&page.events);
    save_csv(&app, "portwatch-history.csv", &csv_text)
}

// ---- alerts -----------------------------------------------------------

#[tauri::command]
pub async fn list_alerts(state: S<'_>, limit: Option<u32>) -> CmdResult<Vec<Alert>> {
    state.db.list_alerts(limit.unwrap_or(500))
}

#[tauri::command]
pub async fn mark_alerts_read(state: S<'_>) -> CmdResult<()> {
    state.db.mark_alerts_read()?;
    state.unread_alerts.store(0, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn clear_alerts(state: S<'_>) -> CmdResult<()> {
    state.db.clear_alerts()?;
    state.unread_alerts.store(0, Ordering::SeqCst);
    Ok(())
}

// ---- settings ---------------------------------------------------------

/// Reports the real Windows startup entry state rather than trusting the saved flag.
fn with_actual_startup(app: &AppHandle, mut s: Settings) -> Settings {
    if let Ok(enabled) = app.autolaunch().is_enabled() {
        s.launch_at_startup = enabled;
    }
    s
}

#[tauri::command]
pub fn get_settings(app: AppHandle, state: S<'_>) -> Settings {
    with_actual_startup(&app, state.settings.get())
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, state: S<'_>, settings: Settings) -> CmdResult<Settings> {
    settings.validate().map_err(|e| e.join(" "))?;
    // Only touch the Windows startup entry when the user actually changed it.
    let actual = app
        .autolaunch()
        .is_enabled()
        .unwrap_or(state.settings.get().launch_at_startup);
    if settings.launch_at_startup != actual {
        let mgr = app.autolaunch();
        let res = if settings.launch_at_startup {
            mgr.enable()
        } else {
            mgr.disable()
        };
        res.map_err(|e| format!("Could not update the Windows startup entry: {e}"))?;
    }
    let saved = state.settings.set(settings)?;
    state.wake();
    Ok(saved)
}

#[tauri::command]
pub async fn reset_settings(app: AppHandle, state: S<'_>) -> CmdResult<Settings> {
    // Resetting turns the startup entry off again (the default).
    if app.autolaunch().is_enabled().unwrap_or(false) {
        app.autolaunch()
            .disable()
            .map_err(|e| format!("Could not remove the Windows startup entry: {e}"))?;
    }
    let s = state.settings.reset()?;
    state.wake();
    Ok(s)
}

// ---- storage ----------------------------------------------------------

#[tauri::command]
pub async fn get_storage_info(state: S<'_>) -> CmdResult<StorageInfo> {
    Ok(StorageInfo {
        data_dir: state.data_dir.display().to_string(),
        database_path: state
            .db
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(in memory only)".into()),
        settings_path: state.settings.path().display().to_string(),
        database_size_bytes: state.db.file_size(),
        event_count: state.db.count_events(),
        alert_count: state.db.count_alerts(),
        portable: state.portable,
        history_in_memory_only: state.db.is_in_memory(),
    })
}

#[tauri::command]
pub async fn open_data_folder(state: S<'_>) -> CmdResult<()> {
    processes::open_folder(&state.data_dir)
}

/// Deletes history, alert history and the memory of previously seen listeners.
#[tauri::command]
pub async fn clear_all_data(state: S<'_>) -> CmdResult<()> {
    state.db.clear_everything()?;
    lock(&state.recent_events).clear();
    state.unread_alerts.store(0, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn open_repository() -> CmdResult<()> {
    processes::open_url(REPOSITORY_URL)
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    tray::quit(&app);
}

#[tauri::command]
pub fn hide_to_tray(app: AppHandle) {
    tray::hide_main(&app);
}
