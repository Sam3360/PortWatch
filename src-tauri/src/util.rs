//! Small helpers shared across modules.

use chrono::{DateTime, Local, SecondsFormat};
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

pub const REPOSITORY_URL: &str = "https://github.com/Sam3360/PortWatch";

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Local-time ISO 8601 timestamp with offset, e.g. `2026-11-09T14:03:21+05:30`.
pub fn format_ts(ms: i64) -> String {
    match DateTime::from_timestamp_millis(ms) {
        Some(utc) => utc
            .with_timezone(&Local)
            .to_rfc3339_opts(SecondsFormat::Secs, false),
        None => String::new(),
    }
}

/// `1.2.3.4:80` or `[::1]:80`. Port 0 is omitted.
pub fn format_endpoint(address: &str, port: u16) -> String {
    if port == 0 {
        return address.to_string();
    }
    if address.contains(':') {
        format!("[{address}]:{port}")
    } else {
        format!("{address}:{port}")
    }
}

/// True for loopback, unspecified and link-local-less "not really remote" addresses.
pub fn is_local_only(address: &str) -> bool {
    let bare = address.split('%').next().unwrap_or(address);
    match bare.parse::<IpAddr>() {
        Ok(ip) => ip.is_loopback() || ip.is_unspecified(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_formatting() {
        assert_eq!(format_endpoint("10.0.0.1", 443), "10.0.0.1:443");
        assert_eq!(format_endpoint("::1", 8080), "[::1]:8080");
        assert_eq!(format_endpoint("0.0.0.0", 0), "0.0.0.0");
    }

    #[test]
    fn local_only_detection() {
        assert!(is_local_only("127.0.0.1"));
        assert!(is_local_only("::1"));
        assert!(is_local_only("0.0.0.0"));
        assert!(!is_local_only("8.8.8.8"));
        assert!(!is_local_only("2001:db8::1"));
        assert!(!is_local_only("not-an-ip"));
    }

    #[test]
    fn timestamp_formatting_is_iso() {
        let s = format_ts(1_700_000_000_000);
        assert!(s.contains('T'));
        assert!(s.len() >= 20);
        assert_eq!(format_ts(i64::MAX), "");
    }
}
