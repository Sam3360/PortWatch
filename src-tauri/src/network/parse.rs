//! Pure parsing of the raw tables returned by `GetExtendedTcpTable` and
//! `GetExtendedUdpTable` (the `*_OWNER_PID` variants).
//!
//! The Windows calls fill a buffer that starts with a `DWORD` entry count and
//! is followed by fixed-size rows. Parsing the bytes ourselves keeps all the
//! unsafe FFI in one tiny function and lets the parsing be unit tested on any
//! platform with hand-built fixtures.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub const TCP4_ROW: usize = 24;
pub const TCP6_ROW: usize = 56;
pub const UDP4_ROW: usize = 12;
pub const UDP6_ROW: usize = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Protocol::Tcp => "TCP",
            Protocol::Udp => "UDP",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
    DeleteTcb,
    Unknown,
}

impl TcpState {
    /// Values of `MIB_TCP_STATE`.
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Closed,
            2 => Self::Listen,
            3 => Self::SynSent,
            4 => Self::SynReceived,
            5 => Self::Established,
            6 => Self::FinWait1,
            7 => Self::FinWait2,
            8 => Self::CloseWait,
            9 => Self::Closing,
            10 => Self::LastAck,
            11 => Self::TimeWait,
            12 => Self::DeleteTcb,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Closed => "Closed",
            Self::Listen => "Listen",
            Self::SynSent => "SYN sent",
            Self::SynReceived => "SYN received",
            Self::Established => "Established",
            Self::FinWait1 => "FIN wait 1",
            Self::FinWait2 => "FIN wait 2",
            Self::CloseWait => "Close wait",
            Self::Closing => "Closing",
            Self::LastAck => "Last ACK",
            Self::TimeWait => "Time wait",
            Self::DeleteTcb => "Delete TCB",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSocket {
    pub protocol: Protocol,
    pub local_addr: IpAddr,
    pub local_scope: u32,
    pub local_port: u16,
    pub remote_addr: Option<IpAddr>,
    pub remote_scope: u32,
    pub remote_port: u16,
    /// `None` for UDP, which has no connection state.
    pub state: Option<TcpState>,
    pub pid: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    TooShort { expected: usize, actual: usize },
    Overflow,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::TooShort { expected, actual } => {
                write!(
                    f,
                    "table buffer too short (needed {expected} bytes, got {actual})"
                )
            }
            ParseError::Overflow => write!(f, "table reported an impossible entry count"),
        }
    }
}

impl std::error::Error for ParseError {}

fn u32_at(buf: &[u8], offset: usize) -> u32 {
    buf.get(offset..offset + 4)
        .and_then(|s| <[u8; 4]>::try_from(s).ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0)
}

fn ip4_at(buf: &[u8], offset: usize) -> Ipv4Addr {
    // The address is stored in network byte order, so the in-memory byte order
    // is already the octet order.
    match buf
        .get(offset..offset + 4)
        .and_then(|s| <[u8; 4]>::try_from(s).ok())
    {
        Some(b) => Ipv4Addr::from(b),
        None => Ipv4Addr::UNSPECIFIED,
    }
}

fn ip6_at(buf: &[u8], offset: usize) -> Ipv6Addr {
    match buf
        .get(offset..offset + 16)
        .and_then(|s| <[u8; 16]>::try_from(s).ok())
    {
        Some(b) => Ipv6Addr::from(b),
        None => Ipv6Addr::UNSPECIFIED,
    }
}

/// Ports live in the low 16 bits of a `DWORD` in network byte order.
pub fn port_from_dword(dw: u32) -> u16 {
    let lo = (dw & 0xFF) as u16;
    let hi = ((dw >> 8) & 0xFF) as u16;
    (lo << 8) | hi
}

fn entry_count(buf: &[u8], row_size: usize) -> Result<usize, ParseError> {
    if buf.len() < 4 {
        return Err(ParseError::TooShort {
            expected: 4,
            actual: buf.len(),
        });
    }
    let count = u32_at(buf, 0) as usize;
    let needed = count
        .checked_mul(row_size)
        .and_then(|n| n.checked_add(4))
        .ok_or(ParseError::Overflow)?;
    if buf.len() < needed {
        return Err(ParseError::TooShort {
            expected: needed,
            actual: buf.len(),
        });
    }
    Ok(count)
}

pub fn parse_tcp4(buf: &[u8]) -> Result<Vec<RawSocket>, ParseError> {
    let count = entry_count(buf, TCP4_ROW)?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = 4 + i * TCP4_ROW;
        let state = TcpState::from_u32(u32_at(buf, o));
        let listening = state == TcpState::Listen;
        out.push(RawSocket {
            protocol: Protocol::Tcp,
            local_addr: IpAddr::V4(ip4_at(buf, o + 4)),
            local_scope: 0,
            local_port: port_from_dword(u32_at(buf, o + 8)),
            remote_addr: if listening {
                None
            } else {
                Some(IpAddr::V4(ip4_at(buf, o + 12)))
            },
            remote_scope: 0,
            remote_port: if listening {
                0
            } else {
                port_from_dword(u32_at(buf, o + 16))
            },
            state: Some(state),
            pid: u32_at(buf, o + 20),
        });
    }
    Ok(out)
}

pub fn parse_tcp6(buf: &[u8]) -> Result<Vec<RawSocket>, ParseError> {
    let count = entry_count(buf, TCP6_ROW)?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = 4 + i * TCP6_ROW;
        let state = TcpState::from_u32(u32_at(buf, o + 48));
        let listening = state == TcpState::Listen;
        out.push(RawSocket {
            protocol: Protocol::Tcp,
            local_addr: IpAddr::V6(ip6_at(buf, o)),
            local_scope: u32_at(buf, o + 16),
            local_port: port_from_dword(u32_at(buf, o + 20)),
            remote_addr: if listening {
                None
            } else {
                Some(IpAddr::V6(ip6_at(buf, o + 24)))
            },
            remote_scope: if listening { 0 } else { u32_at(buf, o + 40) },
            remote_port: if listening {
                0
            } else {
                port_from_dword(u32_at(buf, o + 44))
            },
            state: Some(state),
            pid: u32_at(buf, o + 52),
        });
    }
    Ok(out)
}

pub fn parse_udp4(buf: &[u8]) -> Result<Vec<RawSocket>, ParseError> {
    let count = entry_count(buf, UDP4_ROW)?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = 4 + i * UDP4_ROW;
        out.push(RawSocket {
            protocol: Protocol::Udp,
            local_addr: IpAddr::V4(ip4_at(buf, o)),
            local_scope: 0,
            local_port: port_from_dword(u32_at(buf, o + 4)),
            remote_addr: None,
            remote_scope: 0,
            remote_port: 0,
            state: None,
            pid: u32_at(buf, o + 8),
        });
    }
    Ok(out)
}

pub fn parse_udp6(buf: &[u8]) -> Result<Vec<RawSocket>, ParseError> {
    let count = entry_count(buf, UDP6_ROW)?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = 4 + i * UDP6_ROW;
        out.push(RawSocket {
            protocol: Protocol::Udp,
            local_addr: IpAddr::V6(ip6_at(buf, o)),
            local_scope: u32_at(buf, o + 16),
            local_port: port_from_dword(u32_at(buf, o + 20)),
            remote_addr: None,
            remote_scope: 0,
            remote_port: 0,
            state: None,
            pid: u32_at(buf, o + 24),
        });
    }
    Ok(out)
}

/// Formats an address for display. IPv6 link-local addresses get their zone
/// index (`fe80::1%12`) because the address alone is ambiguous.
pub fn format_ip(addr: &IpAddr, scope: u32) -> String {
    if let IpAddr::V6(v6) = addr {
        let link_local = (v6.segments()[0] & 0xffc0) == 0xfe80;
        if scope != 0 && link_local {
            return format!("{v6}%{scope}");
        }
    }
    addr.to_string()
}

#[cfg(test)]
pub(crate) mod fixtures {
    //! Helpers that build raw table buffers exactly like Windows does.

    pub fn dword_port(port: u16) -> [u8; 4] {
        // network byte order in the low 16 bits
        [(port >> 8) as u8, (port & 0xFF) as u8, 0, 0]
    }

    pub fn table(rows: Vec<Vec<u8>>) -> Vec<u8> {
        let mut buf = (rows.len() as u32).to_le_bytes().to_vec();
        for r in rows {
            buf.extend(r);
        }
        buf
    }

    pub fn tcp4_row(
        state: u32,
        local: [u8; 4],
        lport: u16,
        remote: [u8; 4],
        rport: u16,
        pid: u32,
    ) -> Vec<u8> {
        let mut r = Vec::new();
        r.extend(state.to_le_bytes());
        r.extend(local);
        r.extend(dword_port(lport));
        r.extend(remote);
        r.extend(dword_port(rport));
        r.extend(pid.to_le_bytes());
        r
    }

    pub fn tcp6_row(
        state: u32,
        local: [u8; 16],
        lscope: u32,
        lport: u16,
        remote: [u8; 16],
        rscope: u32,
        rport: u16,
        pid: u32,
    ) -> Vec<u8> {
        let mut r = Vec::new();
        r.extend(local);
        r.extend(lscope.to_le_bytes());
        r.extend(dword_port(lport));
        r.extend(remote);
        r.extend(rscope.to_le_bytes());
        r.extend(dword_port(rport));
        r.extend(state.to_le_bytes());
        r.extend(pid.to_le_bytes());
        r
    }

    pub fn udp4_row(local: [u8; 4], lport: u16, pid: u32) -> Vec<u8> {
        let mut r = Vec::new();
        r.extend(local);
        r.extend(dword_port(lport));
        r.extend(pid.to_le_bytes());
        r
    }

    pub fn udp6_row(local: [u8; 16], scope: u32, lport: u16, pid: u32) -> Vec<u8> {
        let mut r = Vec::new();
        r.extend(local);
        r.extend(scope.to_le_bytes());
        r.extend(dword_port(lport));
        r.extend(pid.to_le_bytes());
        r
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    const LOOPBACK6: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

    #[test]
    fn port_byte_order_is_converted() {
        assert_eq!(port_from_dword(0x0000_5000), 80);
        assert_eq!(port_from_dword(0x0000_BB01), 443);
        assert_eq!(port_from_dword(0x0000_0000), 0);
        assert_eq!(port_from_dword(0xFFFF_FFFF), 0xFFFF);
        for p in [1u16, 22, 8080, 49152, 65535] {
            assert_eq!(port_from_dword(u32::from_le_bytes(dword_port(p))), p);
        }
    }

    #[test]
    fn tcp_states_map_correctly() {
        assert_eq!(TcpState::from_u32(2), TcpState::Listen);
        assert_eq!(TcpState::from_u32(5), TcpState::Established);
        assert_eq!(TcpState::from_u32(11), TcpState::TimeWait);
        assert_eq!(TcpState::from_u32(99), TcpState::Unknown);
        assert_eq!(TcpState::Established.label(), "Established");
    }

    #[test]
    fn parses_tcp4_established_and_listening() {
        let buf = table(vec![
            tcp4_row(5, [192, 168, 1, 20], 51234, [142, 250, 80, 46], 443, 1234),
            tcp4_row(2, [0, 0, 0, 0], 135, [0, 0, 0, 0], 0, 900),
        ]);
        let socks = parse_tcp4(&buf).unwrap();
        assert_eq!(socks.len(), 2);
        let a = &socks[0];
        assert_eq!(a.local_addr.to_string(), "192.168.1.20");
        assert_eq!(a.local_port, 51234);
        assert_eq!(a.remote_addr.unwrap().to_string(), "142.250.80.46");
        assert_eq!(a.remote_port, 443);
        assert_eq!(a.state, Some(TcpState::Established));
        assert_eq!(a.pid, 1234);
        let l = &socks[1];
        assert_eq!(l.state, Some(TcpState::Listen));
        assert!(l.remote_addr.is_none());
        assert_eq!(l.remote_port, 0);
        assert_eq!(l.local_port, 135);
    }

    #[test]
    fn parses_tcp6_with_scope_and_loopback() {
        let mut ll = [0u8; 16];
        ll[0] = 0xfe;
        ll[1] = 0x80;
        ll[15] = 0x05;
        let buf = table(vec![
            tcp6_row(5, LOOPBACK6, 0, 5000, LOOPBACK6, 0, 5001, 77),
            tcp6_row(2, ll, 12, 445, [0; 16], 0, 0, 4),
        ]);
        let socks = parse_tcp6(&buf).unwrap();
        assert_eq!(socks.len(), 2);
        assert_eq!(socks[0].local_addr.to_string(), "::1");
        assert_eq!(socks[0].remote_port, 5001);
        assert_eq!(
            format_ip(&socks[1].local_addr, socks[1].local_scope),
            "fe80::5%12"
        );
        assert_eq!(socks[1].state, Some(TcpState::Listen));
        assert_eq!(socks[1].pid, 4);
    }

    #[test]
    fn parses_udp_tables() {
        let b4 = table(vec![
            udp4_row([0, 0, 0, 0], 5353, 3000),
            udp4_row([127, 0, 0, 1], 1900, 3001),
        ]);
        let u4 = parse_udp4(&b4).unwrap();
        assert_eq!(u4[0].local_port, 5353);
        assert_eq!(u4[1].local_addr.to_string(), "127.0.0.1");
        assert!(u4.iter().all(|s| s.state.is_none() && s.remote_addr.is_none()));

        let b6 = table(vec![udp6_row(LOOPBACK6, 0, 1900, 3002)]);
        let u6 = parse_udp6(&b6).unwrap();
        assert_eq!(u6[0].local_addr.to_string(), "::1");
        assert_eq!(u6[0].pid, 3002);
    }

    #[test]
    fn empty_tables_are_fine() {
        assert!(parse_tcp4(&table(vec![])).unwrap().is_empty());
        assert!(parse_udp6(&table(vec![])).unwrap().is_empty());
    }

    #[test]
    fn truncated_buffers_are_errors_not_panics() {
        assert!(matches!(parse_tcp4(&[]), Err(ParseError::TooShort { .. })));
        assert!(matches!(parse_tcp4(&[1, 0]), Err(ParseError::TooShort { .. })));
        let mut buf = table(vec![tcp4_row(5, [1, 2, 3, 4], 1, [5, 6, 7, 8], 2, 9)]);
        buf.truncate(buf.len() - 1);
        assert!(matches!(parse_tcp4(&buf), Err(ParseError::TooShort { .. })));
        // A count that would overflow the size computation must not panic.
        let huge = u32::MAX.to_le_bytes().to_vec();
        assert!(parse_tcp6(&huge).is_err());
    }

    #[test]
    fn ipv6_formatting() {
        let v4mapped: IpAddr = "::ffff:10.1.2.3".parse().unwrap();
        assert_eq!(format_ip(&v4mapped, 0), "::ffff:10.1.2.3");
        let global: IpAddr = "2001:db8::1".parse().unwrap();
        assert_eq!(
            format_ip(&global, 7),
            "2001:db8::1",
            "scope only shown for link-local"
        );
        let ll: IpAddr = "fe80::1".parse().unwrap();
        assert_eq!(format_ip(&ll, 0), "fe80::1");
        assert_eq!(format_ip(&ll, 3), "fe80::1%3");
        let v4: IpAddr = "10.0.0.5".parse().unwrap();
        assert_eq!(format_ip(&v4, 9), "10.0.0.5");
    }
}
