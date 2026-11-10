//! System tray: show/hide, monitoring status, pause/resume and exit.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::state::{lock, AppState, TrayItems};

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    // Refresh immediately so the window does not show stale data.
    if let Some(state) = app.try_state::<Arc<AppState>>() {
        state.wake();
    }
}

pub fn hide_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

/// Pauses or resumes monitoring and keeps the tray menu text in sync.
pub fn set_paused(state: &AppState, paused: bool) {
    state.paused.store(paused, Ordering::SeqCst);
    if let Some(items) = lock(&state.tray).as_ref() {
        let _ = items.pause.set_text(if paused {
            "Resume monitoring"
        } else {
            "Pause monitoring"
        });
        if paused {
            let _ = items.status.set_text("Monitoring: paused");
        }
    }
    state.wake();
}

pub fn set_status(state: &AppState, text: &str) {
    if state.paused.load(Ordering::SeqCst) {
        return;
    }
    if let Some(items) = lock(&state.tray).as_ref() {
        let _ = items.status.set_text(text);
    }
}

pub fn quit(app: &AppHandle) {
    if let Some(state) = app.try_state::<Arc<AppState>>() {
        state.stop.store(true, Ordering::SeqCst);
        state.wake();
    }
    app.exit(0);
}

pub fn init(app: &AppHandle, state: &Arc<AppState>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show PortWatch", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide PortWatch", true, None::<&str>)?;
    let status = MenuItem::with_id(app, "status", "Monitoring: starting…", false, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause monitoring", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Exit PortWatch", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &hide, &sep1, &status, &pause, &sep2, &quit_item])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("PortWatch")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main(app),
            "hide" => hide_main(app),
            "pause" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    let now_paused = !state.paused.load(Ordering::SeqCst);
                    set_paused(&state, now_paused);
                }
            }
            "quit" => quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;

    *lock(&state.tray) = Some(TrayItems { status, pause });
    Ok(())
}
