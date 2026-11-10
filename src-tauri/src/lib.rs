//! PortWatch: a local-first Windows network monitor.
//!
//! Module map:
//! - `network`       socket enumeration (IP Helper API), diffing, monitor loop
//! - `processes`     process names, paths and start times
//! - `history`       SQLite storage for events, alerts and known listeners
//! - `notifications` neutral-language alert rules with de-duplication
//! - `settings`      validated, persisted user settings
//! - `tray`          system tray integration
//! - `commands`      the only API surface exposed to the frontend

mod commands;
mod csv;
mod history;
mod models;
mod network;
mod notifications;
mod paths;
mod processes;
mod settings;
mod state;
mod tray;
mod util;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use crate::history::Db;
use crate::settings::SettingsStore;
use crate::state::{lock, AppState};

pub fn run() {
    let app = tauri::Builder::default()
        // Must be first: a second launch just brings the existing window forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .setup(|app| {
            let handle = app.handle().clone();

            let default_dir = app.path().app_data_dir()?;
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()));
            let (data_dir, portable) = paths::resolve_data_dir(default_dir, exe_dir.as_deref());
            std::fs::create_dir_all(&data_dir)?;

            let mut warnings = Vec::new();
            let settings = SettingsStore::load(data_dir.join("settings.json"));
            if let Some(w) = &settings.load_warning {
                warnings.push(w.clone());
            }
            let db = match Db::open(&data_dir.join("portwatch.db")) {
                Ok(db) => db,
                Err(e) => {
                    warnings.push(format!(
                        "{e}. History will only be kept in memory until PortWatch restarts."
                    ));
                    Db::open_in_memory().map_err(|e| e.to_string())?
                }
            };

            let state = Arc::new(AppState::new(settings, db, data_dir, portable, warnings));
            app.manage(state.clone());

            if let Err(e) = tray::init(&handle, &state) {
                eprintln!("PortWatch: could not create the tray icon: {e}");
            }

            if !state.settings.get().start_minimized {
                tray::show_main(&handle);
            }

            let join = network::monitor::spawn(handle, state.clone());
            *lock(&state.monitor_handle) = Some(join);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if let Some(state) = window.app_handle().try_state::<Arc<AppState>>() {
                    if state.settings.get().close_behavior == "tray" {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_snapshot,
            commands::set_paused,
            commands::refresh_now,
            commands::get_process_details,
            commands::reveal_process_folder,
            commands::get_recent_events,
            commands::query_history,
            commands::list_history_processes,
            commands::clear_history,
            commands::save_export,
            commands::export_history_csv,
            commands::list_alerts,
            commands::mark_alerts_read,
            commands::clear_alerts,
            commands::get_settings,
            commands::save_settings,
            commands::reset_settings,
            commands::get_storage_info,
            commands::open_data_folder,
            commands::clear_all_data,
            commands::open_repository,
            commands::quit_app,
            commands::hide_to_tray,
        ])
        .build(tauri::generate_context!())
        .expect("error while building PortWatch");

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            // Stop the monitor and give it a moment to finish its current poll.
            if let Some(state) = app_handle.try_state::<Arc<AppState>>() {
                state.stop.store(true, Ordering::SeqCst);
                state.wake();
                if let Some(handle) = lock(&state.monitor_handle).take() {
                    let deadline = std::time::Instant::now() + Duration::from_secs(2);
                    while !handle.is_finished() && std::time::Instant::now() < deadline {
                        std::thread::sleep(Duration::from_millis(25));
                    }
                    if handle.is_finished() {
                        let _ = handle.join();
                    }
                }
            }
        }
    });
}
