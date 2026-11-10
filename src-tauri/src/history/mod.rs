//! Local SQLite storage for connection history, notification history and the
//! small amount of "already seen" state used to avoid duplicate alerts.
//!
//! What is stored: timestamps, event kind, protocol, addresses, ports, TCP state,
//! PID and process *name*. What is deliberately not stored: executable paths,
//! command lines, window titles, hostnames or any payload data.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, Row};

use crate::models::{Alert, HistoryEvent, HistoryFilter, HistoryPage};
use crate::notifications::KnownStore;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts INTEGER NOT NULL,
    kind TEXT NOT NULL,
    protocol TEXT NOT NULL,
    local_address TEXT NOT NULL,
    local_port INTEGER NOT NULL,
    remote_address TEXT,
    remote_port INTEGER,
    state TEXT,
    prev_state TEXT,
    pid INTEGER NOT NULL,
    process_name TEXT
);
CREATE INDEX IF NOT EXISTS idx_events_ts ON events(ts);
CREATE INDEX IF NOT EXISTS idx_events_process ON events(process_name);
CREATE TABLE IF NOT EXISTS alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts INTEGER NOT NULL,
    category TEXT NOT NULL,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    process_name TEXT,
    pid INTEGER NOT NULL DEFAULT 0,
    port INTEGER,
    protocol TEXT,
    read INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_alerts_ts ON alerts(ts);
CREATE TABLE IF NOT EXISTS known_listeners (
    process_name TEXT PRIMARY KEY,
    first_seen INTEGER NOT NULL
);
";

const MAX_ALERTS_KEPT: i64 = 2000;

pub struct Db {
    conn: Mutex<Connection>,
    path: Option<PathBuf>,
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn row_to_event(r: &Row<'_>) -> rusqlite::Result<HistoryEvent> {
    Ok(HistoryEvent {
        id: r.get(0)?,
        ts: r.get(1)?,
        kind: r.get(2)?,
        protocol: r.get(3)?,
        local_address: r.get(4)?,
        local_port: r.get::<_, i64>(5)? as u16,
        remote_address: r.get(6)?,
        remote_port: r.get::<_, Option<i64>>(7)?.map(|p| p as u16),
        state: r.get(8)?,
        prev_state: r.get(9)?,
        pid: r.get::<_, i64>(10)? as u32,
        process_name: r.get(11)?,
    })
}

const EVENT_COLUMNS: &str = "id, ts, kind, protocol, local_address, local_port, remote_address, \
                             remote_port, state, prev_state, pid, process_name";

fn where_clause(f: &HistoryFilter) -> (String, Vec<Value>) {
    let mut clauses: Vec<String> = Vec::new();
    let mut args: Vec<Value> = Vec::new();

    if let Some(search) = f.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{}%", escape_like(search));
        clauses.push(
            "(COALESCE(process_name,'') LIKE ? ESCAPE '\\' \
              OR CAST(pid AS TEXT) LIKE ? ESCAPE '\\' \
              OR local_address LIKE ? ESCAPE '\\' \
              OR COALESCE(remote_address,'') LIKE ? ESCAPE '\\' \
              OR CAST(local_port AS TEXT) LIKE ? ESCAPE '\\' \
              OR COALESCE(CAST(remote_port AS TEXT),'') LIKE ? ESCAPE '\\' \
              OR protocol LIKE ? ESCAPE '\\')"
                .to_string(),
        );
        for _ in 0..7 {
            args.push(Value::Text(like.clone()));
        }
    }
    if let Some(from) = f.from {
        clauses.push("ts >= ?".into());
        args.push(Value::Integer(from));
    }
    if let Some(to) = f.to {
        clauses.push("ts <= ?".into());
        args.push(Value::Integer(to));
    }
    if let Some(p) = f.protocol.as_deref().filter(|p| !p.is_empty() && *p != "all") {
        clauses.push("protocol = ?".into());
        args.push(Value::Text(p.to_uppercase()));
    }
    if let Some(p) = f.process.as_deref().filter(|p| !p.is_empty()) {
        clauses.push("process_name = ? COLLATE NOCASE".into());
        args.push(Value::Text(p.to_string()));
    }
    if let Some(k) = f.kind.as_deref().filter(|k| !k.is_empty() && *k != "all") {
        clauses.push("kind = ?".into());
        args.push(Value::Text(k.to_string()));
    }
    if clauses.is_empty() {
        (String::new(), args)
    } else {
        (format!(" WHERE {}", clauses.join(" AND ")), args)
    }
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("Could not open history database: {e}"))?;
        // WAL + NORMAL sync keeps writes cheap; events are batched per poll anyway.
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| e.to_string())?;
        Self::init(conn, Some(path.to_path_buf()))
    }

    pub fn open_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        Self::init(conn, None)
    }

    fn init(conn: Connection, path: Option<PathBuf>) -> Result<Self, String> {
        conn.execute_batch(SCHEMA)
            .map_err(|e| format!("Could not prepare history database: {e}"))?;
        Ok(Self {
            conn: Mutex::new(conn),
            path,
        })
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn is_in_memory(&self) -> bool {
        self.path.is_none()
    }

    pub fn file_size(&self) -> u64 {
        let Some(p) = &self.path else { return 0 };
        let mut total = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        for ext in ["-wal", "-shm"] {
            let mut os = p.as_os_str().to_owned();
            os.push(ext);
            total += std::fs::metadata(PathBuf::from(os)).map(|m| m.len()).unwrap_or(0);
        }
        total
    }

    // ---- events -------------------------------------------------------

    /// Inserts a batch in one transaction (one disk sync per poll, not per event).
    pub fn insert_events(&self, events: &[HistoryEvent]) -> Result<(), String> {
        if events.is_empty() {
            return Ok(());
        }
        let mut conn = self.lock();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO events (ts, kind, protocol, local_address, local_port, remote_address, \
                     remote_port, state, prev_state, pid, process_name) VALUES (?,?,?,?,?,?,?,?,?,?,?)",
                )
                .map_err(|e| e.to_string())?;
            for e in events {
                stmt.execute(params![
                    e.ts,
                    e.kind,
                    e.protocol,
                    e.local_address,
                    e.local_port as i64,
                    e.remote_address,
                    e.remote_port.map(|p| p as i64),
                    e.state,
                    e.prev_state,
                    e.pid as i64,
                    e.process_name,
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn query_events(&self, filter: &HistoryFilter) -> Result<HistoryPage, String> {
        let conn = self.lock();
        let (where_sql, args) = where_clause(filter);

        let total: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM events{where_sql}"),
                params_from_iter(args.iter()),
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        let limit = filter.limit.map(|l| l.min(1_000_000) as i64).unwrap_or(200);
        let offset = filter.offset.unwrap_or(0) as i64;
        let sql = format!(
            "SELECT {EVENT_COLUMNS} FROM events{where_sql} ORDER BY ts DESC, id DESC LIMIT ? OFFSET ?"
        );
        let mut all_args = args;
        all_args.push(Value::Integer(limit));
        all_args.push(Value::Integer(offset));
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let events = stmt
            .query_map(params_from_iter(all_args.iter()), row_to_event)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(HistoryPage { events, total })
    }

    pub fn distinct_processes(&self) -> Result<Vec<String>, String> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT process_name FROM events WHERE process_name IS NOT NULL \
                 ORDER BY process_name COLLATE NOCASE LIMIT 2000",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    pub fn count_events(&self) -> i64 {
        self.lock()
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
            .unwrap_or(0)
    }

    pub fn clear_events(&self) -> Result<(), String> {
        self.lock()
            .execute("DELETE FROM events", [])
            .map_err(|e| e.to_string())?;
        self.compact();
        Ok(())
    }

    /// Deletes events older than the retention window and trims to `max_events`.
    /// Returns the number of deleted rows.
    pub fn prune(&self, retention_days: u32, max_events: u32, now: i64) -> Result<usize, String> {
        let conn = self.lock();
        let cutoff = now - (retention_days as i64) * 86_400_000;
        let mut deleted = conn
            .execute("DELETE FROM events WHERE ts < ?", params![cutoff])
            .map_err(|e| e.to_string())?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let excess = count - max_events as i64;
        if excess > 0 {
            deleted += conn
                .execute(
                    "DELETE FROM events WHERE id IN (SELECT id FROM events ORDER BY id ASC LIMIT ?)",
                    params![excess],
                )
                .map_err(|e| e.to_string())?;
        }
        conn.execute(
            "DELETE FROM alerts WHERE id NOT IN (SELECT id FROM alerts ORDER BY id DESC LIMIT ?) OR ts < ?",
            params![MAX_ALERTS_KEPT, cutoff],
        )
        .map_err(|e| e.to_string())?;
        Ok(deleted)
    }

    /// Returns freed pages to the OS and truncates the WAL after big deletions.
    pub fn compact(&self) {
        let conn = self.lock();
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;");
    }

    // ---- alerts -------------------------------------------------------

    pub fn insert_alert(&self, a: &Alert) -> Result<i64, String> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO alerts (ts, category, title, message, process_name, pid, port, protocol, read) \
             VALUES (?,?,?,?,?,?,?,?,0)",
            params![
                a.ts,
                a.category,
                a.title,
                a.message,
                a.process_name,
                a.pid as i64,
                a.port.map(|p| p as i64),
                a.protocol
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_alerts(&self, limit: u32) -> Result<Vec<Alert>, String> {
        let conn = self.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, ts, category, title, message, process_name, pid, port, protocol, read \
                 FROM alerts ORDER BY ts DESC, id DESC LIMIT ?",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit.min(5000) as i64], |r| {
                Ok(Alert {
                    id: r.get(0)?,
                    ts: r.get(1)?,
                    category: r.get(2)?,
                    title: r.get(3)?,
                    message: r.get(4)?,
                    process_name: r.get(5)?,
                    pid: r.get::<_, i64>(6)? as u32,
                    port: r.get::<_, Option<i64>>(7)?.map(|p| p as u16),
                    protocol: r.get(8)?,
                    read: r.get::<_, i64>(9)? != 0,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    pub fn unread_alerts(&self) -> u32 {
        self.lock()
            .query_row("SELECT COUNT(*) FROM alerts WHERE read = 0", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap_or(0) as u32
    }

    pub fn count_alerts(&self) -> i64 {
        self.lock()
            .query_row("SELECT COUNT(*) FROM alerts", [], |r| r.get(0))
            .unwrap_or(0)
    }

    pub fn mark_alerts_read(&self) -> Result<(), String> {
        self.lock()
            .execute("UPDATE alerts SET read = 1 WHERE read = 0", [])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn clear_alerts(&self) -> Result<(), String> {
        self.lock()
            .execute("DELETE FROM alerts", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ---- known listeners ----------------------------------------------

    pub fn clear_known(&self) -> Result<(), String> {
        self.lock()
            .execute("DELETE FROM known_listeners", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_everything(&self) -> Result<(), String> {
        {
            let conn = self.lock();
            conn.execute_batch("DELETE FROM events; DELETE FROM alerts; DELETE FROM known_listeners;")
                .map_err(|e| e.to_string())?;
        }
        self.compact();
        Ok(())
    }
}

impl KnownStore for Db {
    fn is_known_listener(&self, name: &str) -> bool {
        self.lock()
            .query_row(
                "SELECT 1 FROM known_listeners WHERE process_name = ?",
                params![name],
                |_| Ok(()),
            )
            .is_ok()
    }

    fn add_known_listener(&self, name: &str, ts: i64) {
        let _ = self.lock().execute(
            "INSERT OR IGNORE INTO known_listeners (process_name, first_seen) VALUES (?, ?)",
            params![name, ts],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::kind;

    fn ev(
        ts: i64,
        k: &str,
        proc_: &str,
        pid: u32,
        proto: &str,
        lport: u16,
        remote: Option<&str>,
    ) -> HistoryEvent {
        HistoryEvent {
            id: 0,
            ts,
            kind: k.into(),
            protocol: proto.into(),
            local_address: "10.0.0.2".into(),
            local_port: lport,
            remote_address: remote.map(String::from),
            remote_port: remote.map(|_| 443),
            state: Some("Established".into()),
            prev_state: None,
            pid,
            process_name: Some(proc_.into()),
        }
    }

    fn seeded() -> Db {
        let db = Db::open_in_memory().unwrap();
        db.insert_events(&[
            ev(
                1_000,
                kind::CONNECTION_OPENED,
                "chrome.exe",
                10,
                "TCP",
                50000,
                Some("1.1.1.1"),
            ),
            ev(
                2_000,
                kind::CONNECTION_CLOSED,
                "chrome.exe",
                10,
                "TCP",
                50000,
                Some("1.1.1.1"),
            ),
            ev(3_000, kind::UDP_OPENED, "dns.exe", 20, "UDP", 5353, None),
            ev(
                4_000,
                kind::LISTENING_STARTED,
                "dev_server.exe",
                30,
                "TCP",
                8080,
                None,
            ),
        ])
        .unwrap();
        db
    }

    #[test]
    fn insert_and_query_newest_first() {
        let db = seeded();
        let page = db.query_events(&HistoryFilter::default()).unwrap();
        assert_eq!(page.total, 4);
        assert_eq!(page.events.len(), 4);
        assert_eq!(page.events[0].ts, 4_000);
        assert_eq!(page.events[3].ts, 1_000);
        assert_eq!(page.events[1].local_port, 5353);
        assert_eq!(page.events[0].remote_address, None);
    }

    #[test]
    fn filters_combine() {
        let db = seeded();
        let f = HistoryFilter {
            protocol: Some("tcp".into()),
            from: Some(1_500),
            ..Default::default()
        };
        let page = db.query_events(&f).unwrap();
        assert_eq!(page.total, 2);
        assert!(page.events.iter().all(|e| e.protocol == "TCP" && e.ts >= 1_500));

        let f = HistoryFilter {
            process: Some("CHROME.EXE".into()),
            ..Default::default()
        };
        assert_eq!(
            db.query_events(&f).unwrap().total,
            2,
            "process filter is case-insensitive"
        );

        let f = HistoryFilter {
            kind: Some(kind::UDP_OPENED.into()),
            ..Default::default()
        };
        assert_eq!(db.query_events(&f).unwrap().total, 1);

        let f = HistoryFilter {
            to: Some(2_000),
            ..Default::default()
        };
        assert_eq!(db.query_events(&f).unwrap().total, 2);
    }

    #[test]
    fn search_matches_name_pid_port_and_address() {
        let db = seeded();
        let count = |s: &str| {
            db.query_events(&HistoryFilter {
                search: Some(s.into()),
                ..Default::default()
            })
            .unwrap()
            .total
        };
        assert_eq!(count("chrome"), 2);
        assert_eq!(count("8080"), 1);
        assert_eq!(count("1.1.1.1"), 2);
        assert_eq!(count("30"), 1);
        assert_eq!(count("nothing-matches"), 0);
    }

    #[test]
    fn search_treats_like_wildcards_literally() {
        let db = seeded();
        let count = |s: &str| {
            db.query_events(&HistoryFilter {
                search: Some(s.into()),
                ..Default::default()
            })
            .unwrap()
            .total
        };
        assert_eq!(count("%"), 0);
        assert_eq!(count("_"), 1, "only dev_server.exe contains a literal underscore");
        assert_eq!(count("'; DROP TABLE events; --"), 0);
        assert_eq!(db.count_events(), 4);
    }

    #[test]
    fn pagination_and_total() {
        let db = seeded();
        let f = HistoryFilter {
            limit: Some(2),
            offset: Some(1),
            ..Default::default()
        };
        let page = db.query_events(&f).unwrap();
        assert_eq!(page.total, 4);
        assert_eq!(page.events.len(), 2);
        assert_eq!(page.events[0].ts, 3_000);
    }

    #[test]
    fn clear_events_empties_the_table() {
        let db = seeded();
        db.clear_events().unwrap();
        assert_eq!(db.count_events(), 0);
        assert_eq!(db.query_events(&HistoryFilter::default()).unwrap().total, 0);
    }

    #[test]
    fn prune_by_age_and_by_count() {
        let db = seeded();
        let day = 86_400_000;
        // "now" is 1 day + 2500 ms after epoch with 1 day retention: events at 1000 and 2000 expire.
        let deleted = db.prune(1, 1_000, day + 2_500).unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(db.count_events(), 2);

        let deleted = db.prune(3650, 1, day + 2_500).unwrap();
        assert_eq!(deleted, 1);
        let page = db.query_events(&HistoryFilter::default()).unwrap();
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0].ts, 4_000, "newest event is kept");
    }

    #[test]
    fn distinct_process_list_is_sorted() {
        let db = seeded();
        assert_eq!(
            db.distinct_processes().unwrap(),
            vec!["chrome.exe", "dev_server.exe", "dns.exe"]
        );
    }

    #[test]
    fn alerts_lifecycle() {
        let db = Db::open_in_memory().unwrap();
        let a = Alert {
            id: 0,
            ts: 5,
            category: "new_listener".into(),
            title: "New listening port detected".into(),
            message: "x".into(),
            process_name: Some("a.exe".into()),
            pid: 1,
            port: Some(80),
            protocol: Some("TCP".into()),
            read: false,
        };
        db.insert_alert(&a).unwrap();
        db.insert_alert(&Alert { ts: 6, ..a.clone() }).unwrap();
        assert_eq!(db.unread_alerts(), 2);
        let list = db.list_alerts(10).unwrap();
        assert_eq!(list[0].ts, 6);
        assert_eq!(list[0].port, Some(80));
        db.mark_alerts_read().unwrap();
        assert_eq!(db.unread_alerts(), 0);
        assert!(db.list_alerts(10).unwrap().iter().all(|a| a.read));
        db.clear_alerts().unwrap();
        assert_eq!(db.count_alerts(), 0);
    }

    #[test]
    fn known_listener_store() {
        let db = Db::open_in_memory().unwrap();
        assert!(!db.is_known_listener("a.exe"));
        db.add_known_listener("a.exe", 1);
        db.add_known_listener("a.exe", 2);
        assert!(db.is_known_listener("a.exe"));
        db.clear_everything().unwrap();
        assert!(!db.is_known_listener("a.exe"));
    }

    #[test]
    fn file_database_persists_between_opens() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("portwatch.db");
        {
            let db = Db::open(&path).unwrap();
            db.insert_events(&[ev(1, kind::UDP_OPENED, "x.exe", 1, "UDP", 1, None)])
                .unwrap();
            assert!(!db.is_in_memory());
        }
        let db = Db::open(&path).unwrap();
        assert_eq!(db.count_events(), 1);
        assert!(db.file_size() > 0);
    }

    #[test]
    fn opening_an_unwritable_path_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("missing-dir").join("x.db");
        assert!(Db::open(&bad).is_err());
    }
}
