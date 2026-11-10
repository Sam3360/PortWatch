//! Data types shared between the monitor, the database and the frontend.
//! Everything serialises to camelCase JSON to match `src/lib/types.ts`.

use serde::{Deserialize, Serialize};

/// Event kinds stored in the history database.
pub mod kind {
    pub const CONNECTION_OPENED: &str = "connection_opened";
    pub const CONNECTION_CLOSED: &str = "connection_closed";
    pub const STATE_CHANGED: &str = "state_changed";
    pub const LISTENING_STARTED: &str = "listening_started";
    pub const LISTENING_STOPPED: &str = "listening_stopped";
    pub const UDP_OPENED: &str = "udp_opened";
    pub const UDP_CLOSED: &str = "udp_closed";
    pub const PROCESS_APPEARED: &str = "process_appeared";
    pub const PROCESS_DISAPPEARED: &str = "process_disappeared";

    /// Human-readable label used in exports and the UI.
    pub fn label(kind: &str) -> &'static str {
        match kind {
            CONNECTION_OPENED => "Connection first detected",
            CONNECTION_CLOSED => "Connection disappeared",
            STATE_CHANGED => "Connection state changed",
            LISTENING_STARTED => "Port started listening",
            LISTENING_STOPPED => "Port stopped listening",
            UDP_OPENED => "UDP port became active",
            UDP_CLOSED => "UDP port no longer active",
            PROCESS_APPEARED => "Process started using the network",
            PROCESS_DISAPPEARED => "Process no longer running",
            _ => "Unknown event",
        }
    }

    /// Kinds that describe ordinary (non-listening) TCP connections. These are
    /// the high-volume events controlled by the "record connections" setting.
    pub fn is_connection_kind(kind: &str) -> bool {
        matches!(kind, CONNECTION_OPENED | CONNECTION_CLOSED | STATE_CHANGED)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    /// Stable identity used to track a connection between polls.
    pub id: String,
    pub protocol: String,
    pub family: String,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: Option<String>,
    pub remote_port: Option<u16>,
    pub state: String,
    /// Owning PID as reported by Windows. 0 means Windows reported no owner.
    pub pid: u32,
    pub process_name: Option<String>,
    /// TCP LISTEN sockets and bound UDP endpoints.
    pub is_listening: bool,
    /// When PortWatch first observed this connection (not the OS start time).
    pub first_seen: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationRecord {
    pub pid: u32,
    pub name: Option<String>,
    pub path: Option<String>,
    pub start_time: Option<i64>,
    pub first_seen: i64,
    pub last_seen: i64,
    pub active: bool,
    pub connection_count: u32,
    pub listening_ports: Vec<u16>,
    pub observed_connections: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub timestamp: i64,
    pub monitoring_since: i64,
    pub interval_ms: u32,
    pub paused: bool,
    pub connections: Vec<Connection>,
    pub applications: Vec<ApplicationRecord>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    pub event_seq: u64,
    pub unread_alerts: u32,
    pub platform_supported: bool,
    pub has_data: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEvent {
    pub id: i64,
    pub ts: i64,
    pub kind: String,
    pub protocol: String,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: Option<String>,
    pub remote_port: Option<u16>,
    pub state: Option<String>,
    pub prev_state: Option<String>,
    pub pid: u32,
    pub process_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryFilter {
    pub search: Option<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub protocol: Option<String>,
    pub process: Option<String>,
    pub kind: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub events: Vec<HistoryEvent>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub id: i64,
    pub ts: i64,
    pub category: String,
    pub title: String,
    pub message: String,
    pub process_name: Option<String>,
    pub pid: u32,
    pub port: Option<u16>,
    pub protocol: Option<String>,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDetails {
    pub pid: u32,
    pub name: Option<String>,
    pub path: Option<String>,
    pub start_time: Option<i64>,
    pub running: bool,
    pub accessible: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub data_dir: String,
    pub database_path: String,
    pub settings_path: String,
    pub database_size_bytes: u64,
    pub event_count: i64,
    pub alert_count: i64,
    pub portable: bool,
    pub history_in_memory_only: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub repository: String,
    pub platform_supported: bool,
    pub portable: bool,
}
