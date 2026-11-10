//! Shared application state, managed by Tauri as `Arc<AppState>`.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

use tauri::menu::MenuItem;
use tauri::Wry;

use crate::history::Db;
use crate::models::{HistoryEvent, Snapshot};
use crate::settings::SettingsStore;

/// Events kept in memory for the Activity page (this session only).
pub const RING_CAPACITY: usize = 5000;

pub struct TrayItems {
    pub status: MenuItem<Wry>,
    pub pause: MenuItem<Wry>,
}

pub struct AppState {
    pub settings: SettingsStore,
    pub db: Db,
    pub data_dir: PathBuf,
    pub portable: bool,
    /// Non-fatal startup problems shown in the UI (corrupt settings, no database, ...).
    pub startup_warnings: Vec<String>,
    pub db_error: Mutex<Option<String>>,
    pub snapshot: RwLock<Snapshot>,
    pub recent_events: Mutex<VecDeque<HistoryEvent>>,
    pub event_seq: AtomicU64,
    pub paused: AtomicBool,
    pub stop: AtomicBool,
    pub unread_alerts: AtomicU32,
    pub tray: Mutex<Option<TrayItems>>,
    pub monitor_handle: Mutex<Option<JoinHandle<()>>>,
    wake: (Mutex<bool>, Condvar),
}

pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl AppState {
    pub fn new(
        settings: SettingsStore,
        db: Db,
        data_dir: PathBuf,
        portable: bool,
        startup_warnings: Vec<String>,
    ) -> Self {
        let unread = db.unread_alerts();
        Self {
            settings,
            db,
            data_dir,
            portable,
            startup_warnings,
            db_error: Mutex::new(None),
            snapshot: RwLock::new(Snapshot::default()),
            recent_events: Mutex::new(VecDeque::with_capacity(512)),
            event_seq: AtomicU64::new(0),
            paused: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            unread_alerts: AtomicU32::new(unread),
            tray: Mutex::new(None),
            monitor_handle: Mutex::new(None),
            wake: (Mutex::new(false), Condvar::new()),
        }
    }

    /// Wakes the monitor thread immediately (settings changed, resume, refresh now, exit).
    pub fn wake(&self) {
        let (flag, cv) = &self.wake;
        *lock(flag) = true;
        cv.notify_all();
    }

    /// Sleeps until `timeout` elapses or [`wake`](Self::wake) is called.
    pub fn wait(&self, timeout: Duration) {
        let (flag, cv) = &self.wake;
        let mut guard = lock(flag);
        if !*guard {
            guard = match cv.wait_timeout(guard, timeout) {
                Ok((g, _)) => g,
                Err(e) => e.into_inner().0,
            };
        }
        *guard = false;
    }

    /// Assigns session-unique ids and appends to the in-memory ring buffer.
    pub fn push_events(&self, events: &mut [HistoryEvent]) {
        if events.is_empty() {
            return;
        }
        let mut ring = lock(&self.recent_events);
        for e in events.iter_mut() {
            e.id = (self.event_seq.fetch_add(1, Ordering::SeqCst) + 1) as i64;
            if ring.len() >= RING_CAPACITY {
                ring.pop_front();
            }
            ring.push_back(e.clone());
        }
    }

    pub fn set_db_error(&self, err: Option<String>) {
        *lock(&self.db_error) = err;
    }
}
