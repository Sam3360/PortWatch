//! The background polling loop.
//!
//! One dedicated thread enumerates sockets every `refresh_interval_ms`, diffs
//! the result against the previous poll, stores events, raises alerts and
//! publishes a snapshot to the frontend. It never touches the UI thread.

use std::collections::{HashMap, HashSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::models::{Alert, ApplicationRecord, Connection, HistoryEvent, Snapshot};
use crate::network::diff::{ConnectionTracker, ProcessTracker};
use crate::network::{self, to_connection};
use crate::notifications::{AlertDraft, AlertEngine, KnownStore};
use crate::processes;
use crate::settings::Settings;
use crate::state::AppState;
use crate::tray;
use crate::util::now_ms;

const MAX_INACTIVE_APPS: usize = 500;
const PRUNE_EVERY: Duration = Duration::from_secs(3600);

struct AppAgg {
    pid: u32,
    name: Option<String>,
    path: Option<String>,
    start_time: Option<i64>,
    first_seen: i64,
    last_seen: i64,
    active: bool,
    connection_count: u32,
    listening_ports: Vec<u16>,
    observed: u64,
    path_attempts: u8,
}

impl AppAgg {
    fn new(pid: u32, name: Option<String>, now: i64) -> Self {
        Self {
            pid,
            name,
            path: None,
            start_time: None,
            first_seen: now,
            last_seen: now,
            active: true,
            connection_count: 0,
            listening_ports: Vec::new(),
            observed: 0,
            path_attempts: 0,
        }
    }
}

struct Monitor {
    state: Arc<AppState>,
    conns: ConnectionTracker,
    procs: ProcessTracker,
    engine: AlertEngine,
    apps: HashMap<u32, AppAgg>,
    since: i64,
    last_paused_published: bool,
    was_paused: bool,
    baseline_done: bool,
}

pub fn spawn(app: AppHandle, state: Arc<AppState>) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("portwatch-monitor".into())
        .spawn(move || run(app, state))
        .expect("failed to start the monitoring thread")
}

fn run(app: AppHandle, state: Arc<AppState>) {
    let mut m = Monitor {
        state: state.clone(),
        conns: ConnectionTracker::new(),
        procs: ProcessTracker::new(),
        engine: AlertEngine::new(),
        apps: HashMap::new(),
        since: now_ms(),
        last_paused_published: false,
        was_paused: false,
        baseline_done: false,
    };
    let mut last_prune: Option<Instant> = None;

    while !state.stop.load(Ordering::SeqCst) {
        if state.paused.load(Ordering::SeqCst) {
            m.was_paused = true;
            if !m.last_paused_published {
                m.last_paused_published = true;
                let mut snap = state.snapshot.read().unwrap_or_else(|e| e.into_inner()).clone();
                snap.paused = true;
                m.publish(&app, snap);
            }
        } else {
            m.last_paused_published = false;
            if m.was_paused {
                // Changes made while paused are not reported as if they just happened.
                m.conns = ConnectionTracker::new();
                m.procs = ProcessTracker::new();
                m.baseline_done = false;
                m.was_paused = false;
            }
            let result = catch_unwind(AssertUnwindSafe(|| m.poll(&app)));
            if result.is_err() {
                eprintln!("PortWatch: monitoring poll panicked; will retry");
                m.publish_error(
                    &app,
                    "An internal error occurred while reading network state. Retrying.".into(),
                );
            }
        }

        if last_prune.map_or(true, |t| t.elapsed() >= PRUNE_EVERY) {
            last_prune = Some(Instant::now());
            let s = state.settings.get();
            if let Err(e) = state
                .db
                .prune(s.history.retention_days, s.history.max_events, now_ms())
            {
                eprintln!("PortWatch: history cleanup failed: {e}");
            }
        }

        let interval = state
            .settings
            .get()
            .refresh_interval_ms
            .max(crate::settings::MIN_INTERVAL_MS);
        state.wait(Duration::from_millis(interval as u64));
    }
}

impl Monitor {
    fn poll(&mut self, app: &AppHandle) {
        let now = now_ms();
        let settings = self.state.settings.get();

        let enumeration = match network::enumerate() {
            Ok(e) => e,
            Err(msg) => {
                self.publish_error(app, msg);
                return;
            }
        };
        let names = processes::process_names();
        let alive: Option<HashSet<u32>> = if names.is_empty() {
            None
        } else {
            Some(names.keys().copied().collect())
        };

        let mut conns: Vec<Connection> = enumeration
            .sockets
            .iter()
            .map(|s| to_connection(s, &names, now))
            .collect();

        let baseline = !self.baseline_done;
        self.baseline_done = true;
        let diff = self.conns.update(&mut conns, now);

        let mut owners: HashMap<u32, Option<String>> = HashMap::new();
        for c in &conns {
            if c.pid != 0 {
                owners.entry(c.pid).or_insert_with(|| c.process_name.clone());
            }
        }
        let mut events = diff.events;
        events.extend(self.procs.update(&owners, alive.as_ref(), now));
        self.update_apps(&conns, &diff.new_by_pid, now);

        if baseline {
            // Existing listeners are "known": only programs that start listening
            // while PortWatch is watching can trigger a new-listener alert.
            for c in &conns {
                if c.protocol == "TCP" && c.is_listening {
                    if let Some(n) = &c.process_name {
                        self.state.db.add_known_listener(&n.to_lowercase(), now);
                    }
                }
            }
        }

        self.state.push_events(&mut events);
        self.store_events(&events, &settings);

        let drafts = self
            .engine
            .evaluate(&events, &settings.notifications, &self.state.db, now);
        self.deliver(app, drafts, &settings, now);

        let snapshot = self.build_snapshot(conns, enumeration.warnings, None, now, &settings);
        let active = snapshot
            .connections
            .iter()
            .filter(|c| c.state == "Established")
            .count();
        let listening = snapshot
            .connections
            .iter()
            .filter(|c| c.protocol == "TCP" && c.is_listening)
            .count();
        tray::set_status(
            &self.state,
            &format!("Monitoring: {active} active, {listening} listening"),
        );
        self.publish(app, snapshot);
    }

    fn store_events(&self, events: &[HistoryEvent], settings: &Settings) {
        let hs = &settings.history;
        let to_store: Vec<HistoryEvent> = events
            .iter()
            .filter(|e| hs.should_record(&e.kind))
            .map(|e| {
                let mut e = e.clone();
                if !hs.record_remote_addresses {
                    e.remote_address = None;
                    e.remote_port = None;
                }
                e
            })
            .collect();
        match self.state.db.insert_events(&to_store) {
            Ok(()) => self.state.set_db_error(None),
            Err(e) => {
                eprintln!("PortWatch: could not write history: {e}");
                self.state
                    .set_db_error(Some(format!("History could not be saved: {e}")));
            }
        }
    }

    fn deliver(&mut self, app: &AppHandle, drafts: Vec<AlertDraft>, settings: &Settings, now: i64) {
        for d in drafts {
            let alert = Alert {
                id: 0,
                ts: d.ts,
                category: d.category.to_string(),
                title: d.title,
                message: d.message,
                process_name: d.process_name,
                pid: d.pid,
                port: d.port,
                protocol: d.protocol,
                read: false,
            };
            match self.state.db.insert_alert(&alert) {
                Ok(_) => {
                    self.state.unread_alerts.fetch_add(1, Ordering::SeqCst);
                }
                Err(e) => eprintln!("PortWatch: could not save alert: {e}"),
            }
            if settings.notifications.desktop_toasts && self.engine.toast_allowed(now) {
                if let Err(e) = app
                    .notification()
                    .builder()
                    .title(alert.title.clone())
                    .body(alert.message.clone())
                    .show()
                {
                    eprintln!("PortWatch: could not show notification: {e}");
                }
            }
        }
    }

    fn update_apps(&mut self, conns: &[Connection], new_by_pid: &HashMap<u32, u32>, now: i64) {
        for rec in self.apps.values_mut() {
            rec.active = false;
            rec.connection_count = 0;
            rec.listening_ports.clear();
        }
        for c in conns {
            if c.pid == 0 {
                continue;
            }
            let rec = self
                .apps
                .entry(c.pid)
                .or_insert_with(|| AppAgg::new(c.pid, c.process_name.clone(), now));
            match (&rec.name, &c.process_name) {
                (Some(old), Some(new)) if old != new => {
                    // PID reused by a different executable: start a fresh record.
                    *rec = AppAgg::new(c.pid, c.process_name.clone(), now);
                }
                (None, Some(_)) => rec.name = c.process_name.clone(),
                _ => {}
            }
            rec.active = true;
            rec.last_seen = now;
            rec.connection_count += 1;
            if c.is_listening {
                rec.listening_ports.push(c.local_port);
            }
        }
        for (pid, n) in new_by_pid {
            if let Some(rec) = self.apps.get_mut(pid) {
                rec.observed += *n as u64;
            }
        }
        for rec in self.apps.values_mut() {
            rec.listening_ports.sort_unstable();
            rec.listening_ports.dedup();
            // Path and start time are looked up once per process (a few retries
            // if Windows refuses), not on every poll.
            if rec.active && rec.path.is_none() && rec.path_attempts < 3 {
                rec.path_attempts += 1;
                let q = processes::query(rec.pid);
                rec.path = q.path;
                if rec.start_time.is_none() {
                    rec.start_time = q.start_time;
                }
            }
        }
        let inactive = self.apps.values().filter(|a| !a.active).count();
        if inactive > MAX_INACTIVE_APPS {
            let mut old: Vec<(u32, i64)> = self
                .apps
                .values()
                .filter(|a| !a.active)
                .map(|a| (a.pid, a.last_seen))
                .collect();
            old.sort_by_key(|(_, seen)| *seen);
            for (pid, _) in old.into_iter().take(inactive - MAX_INACTIVE_APPS) {
                self.apps.remove(&pid);
            }
        }
    }

    fn build_snapshot(
        &self,
        connections: Vec<Connection>,
        mut warnings: Vec<String>,
        error: Option<String>,
        now: i64,
        settings: &Settings,
    ) -> Snapshot {
        warnings.extend(self.state.startup_warnings.iter().cloned());
        if let Some(e) = self
            .state
            .db_error
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        {
            warnings.push(e);
        }
        let mut applications: Vec<ApplicationRecord> = self
            .apps
            .values()
            .map(|a| ApplicationRecord {
                pid: a.pid,
                name: a.name.clone(),
                path: a.path.clone(),
                start_time: a.start_time,
                first_seen: a.first_seen,
                last_seen: a.last_seen,
                active: a.active,
                connection_count: a.connection_count,
                listening_ports: a.listening_ports.clone(),
                observed_connections: a.observed,
            })
            .collect();
        applications.sort_by(|a, b| {
            b.active
                .cmp(&a.active)
                .then(b.connection_count.cmp(&a.connection_count))
                .then(a.name.cmp(&b.name))
        });
        Snapshot {
            timestamp: now,
            monitoring_since: self.since,
            interval_ms: settings.refresh_interval_ms,
            paused: self.state.paused.load(Ordering::SeqCst),
            connections,
            applications,
            warnings,
            error,
            event_seq: self.state.event_seq.load(Ordering::SeqCst),
            unread_alerts: self.state.unread_alerts.load(Ordering::SeqCst),
            platform_supported: cfg!(windows),
            has_data: true,
        }
    }

    fn publish_error(&mut self, app: &AppHandle, message: String) {
        let mut snap = self
            .state
            .snapshot
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        snap.error = Some(message);
        snap.timestamp = now_ms();
        snap.platform_supported = cfg!(windows);
        snap.interval_ms = self.state.settings.get().refresh_interval_ms;
        snap.monitoring_since = self.since;
        snap.paused = self.state.paused.load(Ordering::SeqCst);
        self.publish(app, snap);
    }

    fn publish(&self, app: &AppHandle, snapshot: Snapshot) {
        *self.state.snapshot.write().unwrap_or_else(|e| e.into_inner()) = snapshot.clone();
        // Skip the IPC work while the window is hidden in the tray.
        let visible = app
            .get_webview_window("main")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);
        if visible {
            if let Err(e) = app.emit("snapshot-updated", &snapshot) {
                eprintln!("PortWatch: could not emit snapshot: {e}");
            }
        }
    }
}
