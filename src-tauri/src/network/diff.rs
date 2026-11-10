//! Compares consecutive polls and produces history events.
//!
//! This module is pure: it knows nothing about Windows, the database or the UI,
//! which makes it easy to test with hand-made `Connection` lists.

use std::collections::{HashMap, HashSet};

use crate::models::{kind, Connection, HistoryEvent};

fn is_terminal(state: &str) -> bool {
    matches!(state, "Time wait" | "Closed" | "Delete TCB")
}

struct Tracked {
    first_seen: i64,
    conn: Connection,
    /// Last non-zero owner. Windows reports PID 0 for TIME_WAIT sockets, but the
    /// close event should still name the process that really owned the connection.
    owner_pid: u32,
    owner_name: Option<String>,
    /// True once a "connection disappeared" event was produced (TIME_WAIT).
    close_reported: bool,
}

pub struct DiffOutput {
    pub events: Vec<HistoryEvent>,
    /// Number of newly seen sockets per owning PID (including the first poll).
    pub new_by_pid: HashMap<u32, u32>,
}

#[derive(Default)]
pub struct ConnectionTracker {
    tracked: HashMap<String, Tracked>,
    initialised: bool,
}

fn event(
    ts: i64,
    event_kind: &str,
    c: &Connection,
    pid: u32,
    name: Option<String>,
    state: Option<String>,
    prev_state: Option<String>,
) -> HistoryEvent {
    HistoryEvent {
        id: 0,
        ts,
        kind: event_kind.to_string(),
        protocol: c.protocol.clone(),
        local_address: c.local_address.clone(),
        local_port: c.local_port,
        remote_address: c.remote_address.clone(),
        remote_port: c.remote_port,
        state,
        prev_state,
        pid,
        process_name: name,
    }
}

fn open_kind(c: &Connection) -> &'static str {
    if c.protocol == "UDP" {
        kind::UDP_OPENED
    } else if c.is_listening {
        kind::LISTENING_STARTED
    } else {
        kind::CONNECTION_OPENED
    }
}

fn close_kind(c: &Connection) -> &'static str {
    if c.protocol == "UDP" {
        kind::UDP_CLOSED
    } else if c.is_listening {
        kind::LISTENING_STOPPED
    } else {
        kind::CONNECTION_CLOSED
    }
}

impl ConnectionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates `first_seen` on each connection and returns the events since the
    /// previous call. The very first call is a baseline: sockets that already
    /// exist are tracked but not reported as new.
    pub fn update(&mut self, conns: &mut [Connection], now: i64) -> DiffOutput {
        let baseline = !self.initialised;
        self.initialised = true;
        let mut events = Vec::new();
        let mut new_by_pid: HashMap<u32, u32> = HashMap::new();
        let mut seen: HashSet<String> = HashSet::with_capacity(conns.len());

        for c in conns.iter_mut() {
            seen.insert(c.id.clone());
            match self.tracked.get_mut(&c.id) {
                Some(t) => {
                    c.first_seen = t.first_seen;
                    if c.pid != 0 {
                        t.owner_pid = c.pid;
                        t.owner_name = c.process_name.clone();
                    }
                    if t.conn.state != c.state && c.protocol == "TCP" && !c.is_listening {
                        if is_terminal(&c.state) {
                            if !t.close_reported {
                                events.push(event(
                                    now,
                                    kind::CONNECTION_CLOSED,
                                    c,
                                    t.owner_pid,
                                    t.owner_name.clone(),
                                    Some(c.state.clone()),
                                    Some(t.conn.state.clone()),
                                ));
                                t.close_reported = true;
                            }
                        } else {
                            events.push(event(
                                now,
                                kind::STATE_CHANGED,
                                c,
                                if c.pid != 0 { c.pid } else { t.owner_pid },
                                if c.pid != 0 {
                                    c.process_name.clone()
                                } else {
                                    t.owner_name.clone()
                                },
                                Some(c.state.clone()),
                                Some(t.conn.state.clone()),
                            ));
                        }
                    }
                    t.conn = c.clone();
                }
                None => {
                    c.first_seen = now;
                    let terminal = c.protocol == "TCP" && is_terminal(&c.state);
                    if !terminal {
                        if !baseline {
                            events.push(event(
                                now,
                                open_kind(c),
                                c,
                                c.pid,
                                c.process_name.clone(),
                                Some(c.state.clone()),
                                None,
                            ));
                        }
                        if c.pid != 0 {
                            *new_by_pid.entry(c.pid).or_default() += 1;
                        }
                    }
                    self.tracked.insert(
                        c.id.clone(),
                        Tracked {
                            first_seen: now,
                            conn: c.clone(),
                            owner_pid: c.pid,
                            owner_name: c.process_name.clone(),
                            close_reported: terminal,
                        },
                    );
                }
            }
        }

        let gone: Vec<String> = self
            .tracked
            .keys()
            .filter(|k| !seen.contains(k.as_str()))
            .cloned()
            .collect();
        for key in gone {
            if let Some(t) = self.tracked.remove(&key) {
                if !t.close_reported && !(t.conn.protocol == "TCP" && is_terminal(&t.conn.state)) {
                    events.push(event(
                        now,
                        close_kind(&t.conn),
                        &t.conn,
                        t.owner_pid,
                        t.owner_name.clone(),
                        Some(t.conn.state.clone()),
                        None,
                    ));
                }
            }
        }

        DiffOutput { events, new_by_pid }
    }
}

struct ProcState {
    name: Option<String>,
}

/// Tracks which processes own network sockets and when they stop existing.
#[derive(Default)]
pub struct ProcessTracker {
    known: HashMap<u32, ProcState>,
    initialised: bool,
}

fn process_event(ts: i64, event_kind: &str, pid: u32, name: Option<String>) -> HistoryEvent {
    HistoryEvent {
        id: 0,
        ts,
        kind: event_kind.to_string(),
        protocol: String::new(),
        local_address: String::new(),
        local_port: 0,
        remote_address: None,
        remote_port: None,
        state: None,
        prev_state: None,
        pid,
        process_name: name,
    }
}

impl ProcessTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// `owners` maps every PID that currently owns a socket to its name.
    /// `alive` is the set of all running PIDs, or `None` if the process list
    /// could not be read (in which case no "disappeared" events are produced).
    pub fn update(
        &mut self,
        owners: &HashMap<u32, Option<String>>,
        alive: Option<&HashSet<u32>>,
        now: i64,
    ) -> Vec<HistoryEvent> {
        let baseline = !self.initialised;
        self.initialised = true;
        let mut events = Vec::new();

        for (pid, name) in owners {
            let renamed = match (self.known.get(pid), name) {
                (Some(ProcState { name: Some(old) }), Some(new)) => old != new,
                _ => false,
            };
            if renamed {
                // The PID was reused by a different executable between polls.
                if let Some(old) = self.known.remove(pid) {
                    events.push(process_event(now, kind::PROCESS_DISAPPEARED, *pid, old.name));
                }
            }
            if !self.known.contains_key(pid) {
                self.known.insert(*pid, ProcState { name: name.clone() });
                if !baseline {
                    events.push(process_event(now, kind::PROCESS_APPEARED, *pid, name.clone()));
                }
            } else if let Some(state) = self.known.get_mut(pid) {
                if state.name.is_none() {
                    state.name = name.clone();
                }
            }
        }

        let absent: Vec<u32> = self
            .known
            .keys()
            .filter(|pid| !owners.contains_key(pid))
            .copied()
            .collect();
        for pid in absent {
            match alive {
                Some(alive) if alive.contains(&pid) => {} // still running, just no sockets right now
                Some(_) => {
                    if let Some(state) = self.known.remove(&pid) {
                        events.push(process_event(now, kind::PROCESS_DISAPPEARED, pid, state.name));
                    }
                }
                None => {
                    self.known.remove(&pid);
                }
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tcp(local_port: u16, remote_port: u16, state: &str, pid: u32, name: Option<&str>) -> Connection {
        let listening = state == "Listen";
        Connection {
            id: if listening {
                format!("TCP|0.0.0.0|{local_port}|L|{pid}")
            } else {
                format!("TCP|10.0.0.2|{local_port}|1.1.1.1|{remote_port}")
            },
            protocol: "TCP".into(),
            family: "IPv4".into(),
            local_address: if listening {
                "0.0.0.0".into()
            } else {
                "10.0.0.2".into()
            },
            local_port,
            remote_address: if listening { None } else { Some("1.1.1.1".into()) },
            remote_port: if listening { None } else { Some(remote_port) },
            state: state.into(),
            pid,
            process_name: name.map(String::from),
            is_listening: listening,
            first_seen: 0,
        }
    }

    fn udp(port: u16, pid: u32) -> Connection {
        Connection {
            id: format!("UDP|0.0.0.0|{port}|{pid}"),
            protocol: "UDP".into(),
            family: "IPv4".into(),
            local_address: "0.0.0.0".into(),
            local_port: port,
            remote_address: None,
            remote_port: None,
            state: "Bound".into(),
            pid,
            process_name: Some("svc.exe".into()),
            is_listening: true,
            first_seen: 0,
        }
    }

    #[test]
    fn first_poll_is_a_silent_baseline() {
        let mut t = ConnectionTracker::new();
        let mut conns = vec![
            tcp(80, 0, "Listen", 4, Some("System")),
            tcp(5000, 443, "Established", 10, Some("a.exe")),
        ];
        let out = t.update(&mut conns, 1000);
        assert!(out.events.is_empty());
        assert_eq!(out.new_by_pid.get(&10), Some(&1));
        assert!(conns.iter().all(|c| c.first_seen == 1000));
    }

    #[test]
    fn detects_new_connections_and_keeps_first_seen() {
        let mut t = ConnectionTracker::new();
        t.update(&mut [], 1000);
        let mut conns = vec![tcp(5000, 443, "Established", 10, Some("a.exe"))];
        let out = t.update(&mut conns, 2000);
        assert_eq!(out.events.len(), 1);
        assert_eq!(out.events[0].kind, kind::CONNECTION_OPENED);
        assert_eq!(out.events[0].process_name.as_deref(), Some("a.exe"));

        let mut again = vec![tcp(5000, 443, "Established", 10, Some("a.exe"))];
        let out2 = t.update(&mut again, 3000);
        assert!(out2.events.is_empty());
        assert_eq!(again[0].first_seen, 2000, "first_seen survives later polls");
    }

    #[test]
    fn detects_listener_start_and_stop() {
        let mut t = ConnectionTracker::new();
        t.update(&mut [], 0);
        let mut conns = vec![tcp(8080, 0, "Listen", 55, Some("dev.exe"))];
        let started = t.update(&mut conns, 10);
        assert_eq!(started.events[0].kind, kind::LISTENING_STARTED);
        assert_eq!(started.events[0].local_port, 8080);
        let stopped = t.update(&mut [], 20);
        assert_eq!(stopped.events.len(), 1);
        assert_eq!(stopped.events[0].kind, kind::LISTENING_STOPPED);
        assert_eq!(stopped.events[0].pid, 55);
    }

    #[test]
    fn udp_endpoints_have_their_own_event_kinds() {
        let mut t = ConnectionTracker::new();
        t.update(&mut [], 0);
        let mut c = vec![udp(5353, 7)];
        assert_eq!(t.update(&mut c, 1).events[0].kind, kind::UDP_OPENED);
        assert_eq!(t.update(&mut [], 2).events[0].kind, kind::UDP_CLOSED);
    }

    #[test]
    fn state_changes_are_reported_with_previous_state() {
        let mut t = ConnectionTracker::new();
        t.update(&mut [], 0);
        let mut c1 = vec![tcp(5000, 443, "SYN sent", 10, Some("a.exe"))];
        t.update(&mut c1, 1);
        let mut c2 = vec![tcp(5000, 443, "Established", 10, Some("a.exe"))];
        let out = t.update(&mut c2, 2);
        assert_eq!(out.events.len(), 1);
        assert_eq!(out.events[0].kind, kind::STATE_CHANGED);
        assert_eq!(out.events[0].prev_state.as_deref(), Some("SYN sent"));
        assert_eq!(out.events[0].state.as_deref(), Some("Established"));
    }

    #[test]
    fn time_wait_counts_as_closed_once_and_keeps_the_real_owner() {
        let mut t = ConnectionTracker::new();
        t.update(&mut [], 0);
        let mut c1 = vec![tcp(5000, 443, "Established", 10, Some("a.exe"))];
        t.update(&mut c1, 1);
        // Windows now reports PID 0 and TIME_WAIT for the same 4-tuple.
        let mut c2 = vec![tcp(5000, 443, "Time wait", 0, None)];
        let out = t.update(&mut c2, 2);
        assert_eq!(out.events.len(), 1);
        assert_eq!(out.events[0].kind, kind::CONNECTION_CLOSED);
        assert_eq!(out.events[0].pid, 10);
        assert_eq!(out.events[0].process_name.as_deref(), Some("a.exe"));
        // When it finally disappears there must be no second "closed" event.
        let out2 = t.update(&mut [], 3);
        assert!(out2.events.is_empty());
    }

    #[test]
    fn connections_first_seen_in_time_wait_are_ignored() {
        let mut t = ConnectionTracker::new();
        t.update(&mut [], 0);
        let mut c = vec![tcp(6000, 80, "Time wait", 0, None)];
        let out = t.update(&mut c, 1);
        assert!(out.events.is_empty());
        assert!(out.new_by_pid.is_empty());
        assert!(t.update(&mut [], 2).events.is_empty());
    }

    #[test]
    fn plain_disappearance_reports_last_known_owner() {
        let mut t = ConnectionTracker::new();
        t.update(&mut [], 0);
        let mut c = vec![tcp(5000, 443, "Established", 10, Some("a.exe"))];
        t.update(&mut c, 1);
        let out = t.update(&mut [], 2);
        assert_eq!(out.events[0].kind, kind::CONNECTION_CLOSED);
        assert_eq!(out.events[0].pid, 10);
    }

    #[test]
    fn process_tracker_reports_appear_and_disappear() {
        let mut p = ProcessTracker::new();
        let mut owners: HashMap<u32, Option<String>> = HashMap::new();
        owners.insert(1, Some("one.exe".into()));
        let alive: HashSet<u32> = [1].into_iter().collect();
        assert!(
            p.update(&owners, Some(&alive), 0).is_empty(),
            "baseline is silent"
        );

        owners.insert(2, Some("two.exe".into()));
        let alive2: HashSet<u32> = [1, 2].into_iter().collect();
        let ev = p.update(&owners, Some(&alive2), 1);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].kind, kind::PROCESS_APPEARED);

        // Process 2 closes its sockets but keeps running: no event.
        owners.remove(&2);
        assert!(p.update(&owners, Some(&alive2), 2).is_empty());

        // Process 2 exits.
        let ev = p.update(&owners, Some(&alive), 3);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].kind, kind::PROCESS_DISAPPEARED);
        assert_eq!(ev[0].process_name.as_deref(), Some("two.exe"));
    }

    #[test]
    fn process_tracker_stays_quiet_if_process_list_is_unavailable() {
        let mut p = ProcessTracker::new();
        let mut owners: HashMap<u32, Option<String>> = HashMap::new();
        owners.insert(1, Some("one.exe".into()));
        p.update(&owners, None, 0);
        owners.clear();
        assert!(p.update(&owners, None, 1).is_empty());
    }

    #[test]
    fn process_tracker_detects_pid_reuse_by_a_different_executable() {
        let mut p = ProcessTracker::new();
        let mut owners: HashMap<u32, Option<String>> = HashMap::new();
        owners.insert(9, Some("old.exe".into()));
        let alive: HashSet<u32> = [9].into_iter().collect();
        p.update(&owners, Some(&alive), 0);
        owners.insert(9, Some("new.exe".into()));
        let ev = p.update(&owners, Some(&alive), 1);
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0].kind, kind::PROCESS_DISAPPEARED);
        assert_eq!(ev[1].kind, kind::PROCESS_APPEARED);
    }
}
