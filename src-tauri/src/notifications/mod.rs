//! Decides which observed events deserve a notification.
//!
//! Wording is intentionally neutral: PortWatch reports what it *observed*
//! ("New listening port detected") and never labels a process as malicious.
//! The decision logic is pure and tested; delivery (database + desktop toast)
//! happens in the monitor.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::models::{kind, HistoryEvent};
use crate::settings::NotificationSettings;
use crate::util::{format_endpoint, is_local_only};

/// The same alert (same category + process + port) is not repeated within this window.
pub const COOLDOWN_MS: i64 = 5 * 60 * 1000;
/// At most this many desktop toasts per minute; the rest are still saved in the alert history.
pub const MAX_TOASTS_PER_MINUTE: usize = 5;
const MAX_SEEN_REMOTES: usize = 10_000;

pub const CAT_NEW_LISTENER: &str = "new_listener";
pub const CAT_NEW_CONNECTION: &str = "new_connection";
pub const CAT_MONITORED_PORT: &str = "monitored_port";
pub const CAT_PROCESS_DISAPPEARED: &str = "process_disappeared";

/// Persistent memory of which process names have been seen listening before.
pub trait KnownStore {
    fn is_known_listener(&self, name: &str) -> bool;
    fn add_known_listener(&self, name: &str, ts: i64);
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlertDraft {
    pub ts: i64,
    pub category: &'static str,
    pub title: String,
    pub message: String,
    pub process_name: Option<String>,
    pub pid: u32,
    pub port: Option<u16>,
    pub protocol: Option<String>,
}

#[derive(Default)]
pub struct AlertEngine {
    cooldowns: HashMap<String, i64>,
    toast_times: VecDeque<i64>,
    seen_remotes: HashSet<String>,
}

fn who(e: &HistoryEvent) -> String {
    match &e.process_name {
        Some(n) => format!("{n} (PID {})", e.pid),
        None => "A process with an unavailable name".to_string(),
    }
}

impl AlertEngine {
    pub fn new() -> Self {
        Self::default()
    }

    fn cooled_down(&mut self, key: String, now: i64) -> bool {
        if let Some(last) = self.cooldowns.get(&key) {
            if now - last < COOLDOWN_MS {
                return false;
            }
        }
        self.cooldowns.insert(key, now);
        if self.cooldowns.len() > 5_000 {
            self.cooldowns.retain(|_, t| now - *t < COOLDOWN_MS);
        }
        true
    }

    /// Returns true if a desktop toast may be shown now (rate limit).
    pub fn toast_allowed(&mut self, now: i64) -> bool {
        while let Some(front) = self.toast_times.front() {
            if now - front > 60_000 {
                self.toast_times.pop_front();
            } else {
                break;
            }
        }
        if self.toast_times.len() >= MAX_TOASTS_PER_MINUTE {
            return false;
        }
        self.toast_times.push_back(now);
        true
    }

    /// Looks at newly produced events and returns the alerts to record.
    /// Listener names are always remembered, even if alerts are switched off,
    /// so enabling alerts later does not produce a flood for old processes.
    pub fn evaluate(
        &mut self,
        events: &[HistoryEvent],
        cfg: &NotificationSettings,
        known: &dyn KnownStore,
        now: i64,
    ) -> Vec<AlertDraft> {
        let mut out = Vec::new();
        for e in events {
            match e.kind.as_str() {
                kind::LISTENING_STARTED => {
                    if let Some(name) = &e.process_name {
                        let key = name.to_lowercase();
                        if !known.is_known_listener(&key) {
                            known.add_known_listener(&key, now);
                            if cfg.enabled
                                && cfg.new_listener
                                && self.cooled_down(format!("{CAT_NEW_LISTENER}|{key}|{}", e.local_port), now)
                            {
                                out.push(AlertDraft {
                                    ts: now,
                                    category: CAT_NEW_LISTENER,
                                    title: "New listening port detected".into(),
                                    message: format!(
                                        "{} started listening on {} (TCP). This program had not been seen listening before.",
                                        who(e),
                                        format_endpoint(&e.local_address, e.local_port)
                                    ),
                                    process_name: e.process_name.clone(),
                                    pid: e.pid,
                                    port: Some(e.local_port),
                                    protocol: Some("TCP".into()),
                                });
                            }
                        }
                    }
                    self.monitored(e, true, cfg, now, &mut out);
                }
                kind::LISTENING_STOPPED => self.monitored(e, false, cfg, now, &mut out),
                kind::UDP_OPENED => self.monitored(e, true, cfg, now, &mut out),
                kind::UDP_CLOSED => self.monitored(e, false, cfg, now, &mut out),
                kind::CONNECTION_OPENED => {
                    if !(cfg.enabled && cfg.new_connection) {
                        continue;
                    }
                    let (Some(name), Some(remote)) = (&e.process_name, &e.remote_address) else {
                        continue;
                    };
                    if is_local_only(remote) {
                        continue;
                    }
                    let rport = e.remote_port.unwrap_or(0);
                    let key = format!("{}|{}|{}", name.to_lowercase(), remote, rport);
                    if self.seen_remotes.len() >= MAX_SEEN_REMOTES {
                        self.seen_remotes.clear();
                    }
                    if !self.seen_remotes.insert(key.clone()) {
                        continue;
                    }
                    if self.cooled_down(format!("{CAT_NEW_CONNECTION}|{key}"), now) {
                        out.push(AlertDraft {
                            ts: now,
                            category: CAT_NEW_CONNECTION,
                            title: "New connection detected".into(),
                            message: format!(
                                "{} connected to {} ({}).",
                                who(e),
                                format_endpoint(remote, rport),
                                e.protocol
                            ),
                            process_name: e.process_name.clone(),
                            pid: e.pid,
                            port: Some(rport),
                            protocol: Some(e.protocol.clone()),
                        });
                    }
                }
                kind::PROCESS_DISAPPEARED => {
                    if !(cfg.enabled && cfg.process_disappeared) {
                        continue;
                    }
                    // Only for programs previously seen listening, so short-lived
                    // helper processes do not generate noise.
                    let Some(name) = &e.process_name else { continue };
                    let lname = name.to_lowercase();
                    if known.is_known_listener(&lname)
                        && self.cooled_down(format!("{CAT_PROCESS_DISAPPEARED}|{lname}"), now)
                    {
                        out.push(AlertDraft {
                            ts: now,
                            category: CAT_PROCESS_DISAPPEARED,
                            title: "Previously observed process is no longer running".into(),
                            message: format!("{} was previously seen listening and has exited.", who(e)),
                            process_name: e.process_name.clone(),
                            pid: e.pid,
                            port: None,
                            protocol: None,
                        });
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn monitored(
        &mut self,
        e: &HistoryEvent,
        active: bool,
        cfg: &NotificationSettings,
        now: i64,
        out: &mut Vec<AlertDraft>,
    ) {
        if !(cfg.enabled && cfg.monitored_port_change) || !cfg.monitored_ports.contains(&e.local_port) {
            return;
        }
        let state = if active {
            "is now active"
        } else {
            "is no longer active"
        };
        let key = format!("{CAT_MONITORED_PORT}|{}|{}|{active}", e.protocol, e.local_port);
        if !self.cooled_down(key, now) {
            return;
        }
        out.push(AlertDraft {
            ts: now,
            category: CAT_MONITORED_PORT,
            title: format!("Monitored port {} changed state", e.local_port),
            message: format!(
                "{} port {} {state}. {}",
                e.protocol,
                e.local_port,
                if e.process_name.is_some() {
                    who(e)
                } else {
                    String::new()
                }
            )
            .trim()
            .to_string(),
            process_name: e.process_name.clone(),
            pid: e.pid,
            port: Some(e.local_port),
            protocol: Some(e.protocol.clone()),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct MemKnown(RefCell<HashSet<String>>);
    impl KnownStore for MemKnown {
        fn is_known_listener(&self, name: &str) -> bool {
            self.0.borrow().contains(name)
        }
        fn add_known_listener(&self, name: &str, _ts: i64) {
            self.0.borrow_mut().insert(name.to_string());
        }
    }

    fn listen(name: Option<&str>, port: u16) -> HistoryEvent {
        HistoryEvent {
            id: 0,
            ts: 0,
            kind: kind::LISTENING_STARTED.into(),
            protocol: "TCP".into(),
            local_address: "0.0.0.0".into(),
            local_port: port,
            remote_address: None,
            remote_port: None,
            state: Some("Listen".into()),
            prev_state: None,
            pid: 77,
            process_name: name.map(String::from),
        }
    }

    fn outbound(name: &str, remote: &str, port: u16) -> HistoryEvent {
        HistoryEvent {
            kind: kind::CONNECTION_OPENED.into(),
            local_address: "10.0.0.2".into(),
            local_port: 50000,
            remote_address: Some(remote.into()),
            remote_port: Some(port),
            state: Some("Established".into()),
            ..listen(Some(name), 50000)
        }
    }

    #[test]
    fn new_listener_alerts_once_per_process_with_neutral_wording() {
        let mut eng = AlertEngine::new();
        let known = MemKnown::default();
        let cfg = NotificationSettings::default();
        let a = eng.evaluate(&[listen(Some("Server.exe"), 8080)], &cfg, &known, 1000);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].title, "New listening port detected");
        assert!(!a[0].message.to_lowercase().contains("malicious"));
        assert!(a[0].message.contains("0.0.0.0:8080"));

        // Same program (case-insensitive) on another port: already known.
        let b = eng.evaluate(&[listen(Some("server.EXE"), 9090)], &cfg, &known, 2000);
        assert!(b.is_empty());
    }

    #[test]
    fn disabled_alerts_still_learn_listeners() {
        let mut eng = AlertEngine::new();
        let known = MemKnown::default();
        let mut cfg = NotificationSettings::default();
        cfg.enabled = false;
        assert!(eng
            .evaluate(&[listen(Some("a.exe"), 1)], &cfg, &known, 0)
            .is_empty());
        cfg.enabled = true;
        assert!(eng
            .evaluate(&[listen(Some("a.exe"), 2)], &cfg, &known, 10)
            .is_empty());
    }

    #[test]
    fn listener_without_a_name_is_not_judged() {
        let mut eng = AlertEngine::new();
        let known = MemKnown::default();
        let a = eng.evaluate(&[listen(None, 4444)], &NotificationSettings::default(), &known, 0);
        assert!(a.is_empty());
    }

    #[test]
    fn monitored_port_changes_alert_and_respect_cooldown() {
        let mut eng = AlertEngine::new();
        let known = MemKnown::default();
        known.add_known_listener("a.exe", 0);
        let mut cfg = NotificationSettings::default();
        cfg.monitored_ports = vec![3389];
        let ev = listen(Some("a.exe"), 3389);
        assert_eq!(eng.evaluate(&[ev.clone()], &cfg, &known, 0).len(), 1);
        assert!(
            eng.evaluate(&[ev.clone()], &cfg, &known, 1_000).is_empty(),
            "duplicate within cooldown"
        );
        assert_eq!(eng.evaluate(&[ev], &cfg, &known, COOLDOWN_MS + 1).len(), 1);

        let mut stopped = listen(Some("a.exe"), 3389);
        stopped.kind = kind::LISTENING_STOPPED.into();
        let a = eng.evaluate(&[stopped], &cfg, &known, 2_000);
        assert_eq!(a.len(), 1);
        assert!(a[0].message.contains("no longer active"));

        // Unmonitored port: nothing.
        known.add_known_listener("b.exe", 0);
        assert!(eng
            .evaluate(&[listen(Some("b.exe"), 1234)], &cfg, &known, 3_000)
            .is_empty());
    }

    #[test]
    fn new_connection_alerts_are_off_by_default_and_deduplicated() {
        let known = MemKnown::default();
        let ev = outbound("a.exe", "8.8.8.8", 443);
        let mut eng = AlertEngine::new();
        assert!(eng
            .evaluate(&[ev.clone()], &NotificationSettings::default(), &known, 0)
            .is_empty());

        let mut cfg = NotificationSettings::default();
        cfg.new_connection = true;
        let first = eng.evaluate(&[ev.clone()], &cfg, &known, 0);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].title, "New connection detected");
        assert!(
            eng.evaluate(&[ev], &cfg, &known, 10_000_000).is_empty(),
            "same destination is only reported once"
        );
        assert!(
            eng.evaluate(&[outbound("a.exe", "127.0.0.1", 80)], &cfg, &known, 0)
                .is_empty(),
            "loopback ignored"
        );
    }

    #[test]
    fn process_disappearance_only_for_known_listeners() {
        let known = MemKnown::default();
        known.add_known_listener("svc.exe", 0);
        let mut cfg = NotificationSettings::default();
        cfg.process_disappeared = true;
        let mut gone = listen(Some("svc.exe"), 0);
        gone.kind = kind::PROCESS_DISAPPEARED.into();
        let mut eng = AlertEngine::new();
        assert_eq!(eng.evaluate(&[gone.clone()], &cfg, &known, 0).len(), 1);

        let mut other = gone;
        other.process_name = Some("helper.exe".into());
        assert!(eng.evaluate(&[other], &cfg, &known, 0).is_empty());
    }

    #[test]
    fn toast_rate_limit() {
        let mut eng = AlertEngine::new();
        for i in 0..MAX_TOASTS_PER_MINUTE {
            assert!(eng.toast_allowed(i as i64), "toast {i}");
        }
        assert!(!eng.toast_allowed(1_000));
        assert!(eng.toast_allowed(61_000), "window slides");
    }
}
