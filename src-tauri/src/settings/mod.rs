//! User settings: defaults, validation and local persistence (`settings.json`).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::models::kind;

pub const MIN_INTERVAL_MS: u32 = 500;
pub const MAX_INTERVAL_MS: u32 = 60_000;
pub const MAX_MONITORED_PORTS: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub desktop_toasts: bool,
    pub new_listener: bool,
    pub new_connection: bool,
    pub monitored_port_change: bool,
    pub process_disappeared: bool,
    pub monitored_ports: Vec<u16>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            desktop_toasts: true,
            new_listener: true,
            new_connection: false,
            monitored_port_change: true,
            process_disappeared: false,
            monitored_ports: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct HistorySettings {
    pub enabled: bool,
    pub retention_days: u32,
    pub max_events: u32,
    /// Record open/close/state events for ordinary TCP connections (high volume).
    pub record_connections: bool,
    /// When false, remote addresses and ports are not written to disk.
    pub record_remote_addresses: bool,
}

impl Default for HistorySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_days: 30,
            max_events: 200_000,
            record_connections: true,
            record_remote_addresses: true,
        }
    }
}

impl HistorySettings {
    /// Whether an event of this kind should be written to the database.
    pub fn should_record(&self, event_kind: &str) -> bool {
        self.enabled && (self.record_connections || !kind::is_connection_kind(event_kind))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub refresh_interval_ms: u32,
    pub start_minimized: bool,
    pub launch_at_startup: bool,
    /// "tray" hides the window when closed, "exit" quits PortWatch.
    pub close_behavior: String,
    /// "dark", "light" or "system".
    pub theme: String,
    pub notifications: NotificationSettings,
    pub history: HistorySettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_interval_ms: 2000,
            start_minimized: false,
            launch_at_startup: false,
            close_behavior: "tray".into(),
            theme: "dark".into(),
            notifications: NotificationSettings::default(),
            history: HistorySettings::default(),
        }
    }
}

impl Settings {
    /// Returns every problem found so the UI can show them all at once.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if !(MIN_INTERVAL_MS..=MAX_INTERVAL_MS).contains(&self.refresh_interval_ms) {
            errors.push(format!(
                "Refresh interval must be between {MIN_INTERVAL_MS} and {MAX_INTERVAL_MS} ms."
            ));
        }
        if !matches!(self.close_behavior.as_str(), "tray" | "exit") {
            errors.push("Close behavior must be \"tray\" or \"exit\".".into());
        }
        if !matches!(self.theme.as_str(), "dark" | "light" | "system") {
            errors.push("Theme must be dark, light or system.".into());
        }
        if !(1..=3650).contains(&self.history.retention_days) {
            errors.push("History retention must be between 1 and 3650 days.".into());
        }
        if !(1_000..=5_000_000).contains(&self.history.max_events) {
            errors.push("Maximum history events must be between 1,000 and 5,000,000.".into());
        }
        if self.notifications.monitored_ports.len() > MAX_MONITORED_PORTS {
            errors.push(format!(
                "At most {MAX_MONITORED_PORTS} monitored ports are allowed."
            ));
        }
        if self.notifications.monitored_ports.contains(&0) {
            errors.push("Monitored ports must be between 1 and 65535.".into());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Removes duplicate monitored ports and keeps them sorted.
    pub fn normalise(&mut self) {
        self.notifications.monitored_ports.sort_unstable();
        self.notifications.monitored_ports.dedup();
    }
}

/// Reads settings from disk. Missing files give defaults silently; unreadable
/// or invalid files give defaults plus a message describing what went wrong.
pub fn load_from(path: &Path) -> (Settings, Option<String>) {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Settings::default(), None),
        Err(e) => {
            return (
                Settings::default(),
                Some(format!("Could not read settings file: {e}. Using defaults.")),
            )
        }
    };
    match serde_json::from_str::<Settings>(&text) {
        Ok(mut s) => match s.validate() {
            Ok(()) => {
                s.normalise();
                (s, None)
            }
            Err(errs) => (
                Settings::default(),
                Some(format!(
                    "Saved settings were invalid ({}). Using defaults.",
                    errs.join(" ")
                )),
            ),
        },
        Err(e) => (
            Settings::default(),
            Some(format!("Settings file is corrupt: {e}. Using defaults.")),
        ),
    }
}

/// Writes atomically (temp file + rename) so a crash cannot leave a half-written file.
pub fn save_to(path: &Path, settings: &Settings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|e| format!("Could not write settings: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("Could not save settings: {e}"))
}

pub struct SettingsStore {
    path: PathBuf,
    current: RwLock<Settings>,
    pub load_warning: Option<String>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self {
        let (settings, load_warning) = load_from(&path);
        Self {
            path,
            current: RwLock::new(settings),
            load_warning,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self) -> Settings {
        self.current.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Validates, persists, then applies. Nothing changes if any step fails.
    pub fn set(&self, mut settings: Settings) -> Result<Settings, String> {
        settings.validate().map_err(|e| e.join(" "))?;
        settings.normalise();
        save_to(&self.path, &settings)?;
        *self.current.write().unwrap_or_else(|e| e.into_inner()) = settings.clone();
        Ok(settings)
    }

    pub fn reset(&self) -> Result<Settings, String> {
        self.set(Settings::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_opt_in() {
        let s = Settings::default();
        assert!(s.validate().is_ok());
        assert!(!s.launch_at_startup, "startup must be opt-in");
        assert!(!s.start_minimized);
        assert_eq!(s.theme, "dark");
    }

    #[test]
    fn validation_reports_all_problems() {
        let mut s = Settings::default();
        s.refresh_interval_ms = 10;
        s.theme = "neon".into();
        s.history.retention_days = 0;
        s.notifications.monitored_ports = vec![0];
        let errs = s.validate().unwrap_err();
        assert_eq!(errs.len(), 4);
    }

    #[test]
    fn settings_round_trip_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = SettingsStore::load(path.clone());
        let mut s = store.get();
        s.refresh_interval_ms = 5000;
        s.notifications.monitored_ports = vec![443, 80, 443];
        store.set(s).unwrap();

        let reloaded = SettingsStore::load(path);
        assert!(reloaded.load_warning.is_none());
        let got = reloaded.get();
        assert_eq!(got.refresh_interval_ms, 5000);
        assert_eq!(got.notifications.monitored_ports, vec![80, 443]);
    }

    #[test]
    fn invalid_settings_are_rejected_and_not_persisted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = SettingsStore::load(path.clone());
        let mut bad = Settings::default();
        bad.refresh_interval_ms = 99_999_999;
        assert!(store.set(bad).is_err());
        assert_eq!(store.get().refresh_interval_ms, 2000);
        assert!(!path.exists());
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults_with_message() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "{ not json").unwrap();
        let (s, warn) = load_from(&path);
        assert_eq!(s, Settings::default());
        assert!(warn.unwrap().contains("corrupt"));
    }

    #[test]
    fn partial_file_uses_defaults_for_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{ "theme": "light" }"#).unwrap();
        let (s, warn) = load_from(&path);
        assert!(warn.is_none());
        assert_eq!(s.theme, "light");
        assert_eq!(s.refresh_interval_ms, 2000);
    }

    #[test]
    fn reset_restores_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::load(dir.path().join("settings.json"));
        let mut s = store.get();
        s.theme = "light".into();
        store.set(s).unwrap();
        let reset = store.reset().unwrap();
        assert_eq!(reset, Settings::default());
    }

    #[test]
    fn history_recording_rules() {
        let mut h = HistorySettings::default();
        assert!(h.should_record(kind::CONNECTION_OPENED));
        h.record_connections = false;
        assert!(!h.should_record(kind::CONNECTION_OPENED));
        assert!(!h.should_record(kind::STATE_CHANGED));
        assert!(h.should_record(kind::LISTENING_STARTED));
        h.enabled = false;
        assert!(!h.should_record(kind::LISTENING_STARTED));
    }
}
