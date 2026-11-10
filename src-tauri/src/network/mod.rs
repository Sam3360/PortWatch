//! Network enumeration and change tracking.

pub mod diff;
pub mod monitor;
pub mod parse;
#[cfg(windows)]
mod winapi;

use std::collections::HashMap;

use crate::models::Connection;
use parse::{format_ip, Protocol, RawSocket, TcpState};

pub struct Enumeration {
    pub sockets: Vec<RawSocket>,
    /// Non-fatal problems, e.g. IPv6 tables being unavailable.
    pub warnings: Vec<String>,
}

#[cfg(windows)]
pub fn enumerate() -> Result<Enumeration, String> {
    winapi::enumerate()
}

#[cfg(not(windows))]
pub fn enumerate() -> Result<Enumeration, String> {
    Err("Network enumeration is only supported on Windows.".into())
}

/// Turns a raw socket into the UI-facing model. The process name is only
/// attached when Windows reported an owning PID *and* that PID is in the
/// current process list; otherwise it stays `None` rather than guessing.
pub fn to_connection(s: &RawSocket, names: &HashMap<u32, String>, now: i64) -> Connection {
    let local_address = format_ip(&s.local_addr, s.local_scope);
    let remote_address = s.remote_addr.map(|a| format_ip(&a, s.remote_scope));
    let family = if s.local_addr.is_ipv4() { "IPv4" } else { "IPv6" };
    let (state, is_listening) = match (s.protocol, s.state) {
        (Protocol::Udp, _) => ("Bound".to_string(), true),
        (Protocol::Tcp, Some(st)) => (st.label().to_string(), st == TcpState::Listen),
        (Protocol::Tcp, None) => ("Unknown".to_string(), false),
    };
    let id = connection_key(
        s.protocol,
        is_listening,
        &local_address,
        s.local_port,
        remote_address.as_deref(),
        s.remote_port,
        s.pid,
    );
    Connection {
        id,
        protocol: s.protocol.as_str().to_string(),
        family: family.to_string(),
        local_address,
        local_port: s.local_port,
        remote_port: remote_address.as_ref().map(|_| s.remote_port),
        remote_address,
        state,
        pid: s.pid,
        process_name: if s.pid == 0 {
            None
        } else {
            names.get(&s.pid).cloned()
        },
        is_listening,
        first_seen: now,
    }
}

/// Identity of a socket between polls.
///
/// * A connected TCP socket is uniquely identified by its 4-tuple. The PID is
///   deliberately excluded because Windows reports PID 0 once a connection
///   moves to TIME_WAIT, which must not look like a different connection.
/// * Listeners and UDP endpoints include the PID so that a different process
///   taking over a port is detected.
pub fn connection_key(
    protocol: Protocol,
    is_listening: bool,
    local: &str,
    lport: u16,
    remote: Option<&str>,
    rport: u16,
    pid: u32,
) -> String {
    match (protocol, is_listening) {
        (Protocol::Tcp, true) => format!("TCP|{local}|{lport}|L|{pid}"),
        (Protocol::Tcp, false) => format!("TCP|{local}|{lport}|{}|{rport}", remote.unwrap_or("-")),
        (Protocol::Udp, _) => format!("UDP|{local}|{lport}|{pid}"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse::fixtures::*;
    use super::*;

    #[test]
    fn builds_connections_from_parsed_tables() {
        let buf = table(vec![
            tcp4_row(5, [10, 0, 0, 2], 50000, [93, 184, 216, 34], 443, 100),
            tcp4_row(2, [0, 0, 0, 0], 80, [0, 0, 0, 0], 0, 200),
            tcp4_row(11, [10, 0, 0, 2], 50001, [93, 184, 216, 34], 443, 0),
        ]);
        let socks = parse::parse_tcp4(&buf).unwrap();
        let mut names = HashMap::new();
        names.insert(100, "browser.exe".to_string());
        names.insert(200, "server.exe".to_string());
        let conns: Vec<_> = socks.iter().map(|s| to_connection(s, &names, 1000)).collect();

        assert_eq!(conns[0].process_name.as_deref(), Some("browser.exe"));
        assert_eq!(conns[0].state, "Established");
        assert!(!conns[0].is_listening);
        assert_eq!(conns[0].remote_port, Some(443));

        assert!(conns[1].is_listening);
        assert_eq!(conns[1].remote_address, None);
        assert_eq!(conns[1].remote_port, None);
        assert_eq!(conns[1].process_name.as_deref(), Some("server.exe"));

        // PID 0 is never mapped to a process.
        assert_eq!(conns[2].pid, 0);
        assert_eq!(conns[2].process_name, None);
    }

    #[test]
    fn unknown_pid_does_not_get_a_name() {
        let buf = table(vec![udp4_row([0, 0, 0, 0], 5353, 999)]);
        let socks = parse::parse_udp4(&buf).unwrap();
        let c = to_connection(&socks[0], &HashMap::new(), 0);
        assert_eq!(c.process_name, None);
        assert_eq!(c.state, "Bound");
        assert!(c.is_listening);
        assert_eq!(c.protocol, "UDP");
    }

    #[test]
    fn time_wait_keeps_the_same_key_even_when_pid_becomes_zero() {
        let a = connection_key(Protocol::Tcp, false, "10.0.0.2", 50000, Some("1.1.1.1"), 443, 100);
        let b = connection_key(Protocol::Tcp, false, "10.0.0.2", 50000, Some("1.1.1.1"), 443, 0);
        assert_eq!(a, b);
        let l1 = connection_key(Protocol::Tcp, true, "0.0.0.0", 80, None, 0, 1);
        let l2 = connection_key(Protocol::Tcp, true, "0.0.0.0", 80, None, 0, 2);
        assert_ne!(
            l1, l2,
            "a different process on the same port is a different listener"
        );
    }

    #[test]
    fn ipv6_family_is_reported() {
        let buf = table(vec![udp6_row([0; 16], 0, 5355, 12)]);
        let socks = parse::parse_udp6(&buf).unwrap();
        let c = to_connection(&socks[0], &HashMap::new(), 0);
        assert_eq!(c.family, "IPv6");
        assert_eq!(c.local_address, "::");
    }
}
