//! Minimal RFC 4180 CSV writer.
//!
//! Fields are quoted when needed and CRLF line endings are used. Text that
//! starts with a spreadsheet formula character (`= + - @`, tab, CR) is prefixed
//! with an apostrophe so values such as process names cannot execute as
//! formulas when the file is opened in Excel.

use crate::models::{kind, HistoryEvent};
use crate::util::format_ts;

pub fn escape_field(value: &str) -> String {
    let mut text = value.to_string();
    if let Some(first) = text.chars().next() {
        let risky = matches!(first, '=' | '+' | '-' | '@' | '\t' | '\r');
        if risky && text.parse::<f64>().is_err() {
            text.insert(0, '\'');
        }
    }
    if text.contains(|c: char| matches!(c, ',' | '"' | '\n' | '\r')) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text
    }
}

pub fn write_csv(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push_str(
        &headers
            .iter()
            .map(|h| escape_field(h))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push_str("\r\n");
    for row in rows {
        out.push_str(&row.iter().map(|f| escape_field(f)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    out
}

pub fn history_csv(events: &[HistoryEvent]) -> String {
    let headers = [
        "Time",
        "Event",
        "Application",
        "PID",
        "Protocol",
        "Local address",
        "Local port",
        "Remote address",
        "Remote port",
        "State",
        "Previous state",
    ];
    let rows: Vec<Vec<String>> = events
        .iter()
        .map(|e| {
            vec![
                format_ts(e.ts),
                kind::label(&e.kind).to_string(),
                e.process_name.clone().unwrap_or_default(),
                if e.pid == 0 {
                    String::new()
                } else {
                    e.pid.to_string()
                },
                e.protocol.clone(),
                e.local_address.clone(),
                if e.local_port == 0 {
                    String::new()
                } else {
                    e.local_port.to_string()
                },
                e.remote_address.clone().unwrap_or_default(),
                e.remote_port
                    .filter(|p| *p != 0)
                    .map(|p| p.to_string())
                    .unwrap_or_default(),
                e.state.clone().unwrap_or_default(),
                e.prev_state.clone().unwrap_or_default(),
            ]
        })
        .collect();
    write_csv(&headers, &rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_fields_are_untouched() {
        assert_eq!(escape_field("chrome.exe"), "chrome.exe");
        assert_eq!(escape_field(""), "");
    }

    #[test]
    fn commas_quotes_and_newlines_are_quoted() {
        assert_eq!(escape_field("a,b"), "\"a,b\"");
        assert_eq!(escape_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(escape_field("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn formula_prefixes_are_neutralised_but_numbers_survive() {
        assert_eq!(escape_field("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(escape_field("@cmd"), "'@cmd");
        assert_eq!(escape_field("-5"), "-5");
        assert_eq!(escape_field("+44"), "+44");
    }

    #[test]
    fn rows_use_crlf() {
        let csv = write_csv(&["A", "B"], &[vec!["1".into(), "x,y".into()]]);
        assert_eq!(csv, "A,B\r\n1,\"x,y\"\r\n");
    }

    #[test]
    fn history_export_has_expected_columns_and_values() {
        let ev = HistoryEvent {
            id: 1,
            ts: 1_700_000_000_000,
            kind: kind::LISTENING_STARTED.into(),
            protocol: "TCP".into(),
            local_address: "0.0.0.0".into(),
            local_port: 8080,
            remote_address: None,
            remote_port: None,
            state: Some("Listen".into()),
            prev_state: None,
            pid: 42,
            process_name: Some("my,app.exe".into()),
        };
        let csv = history_csv(&[ev]);
        let mut lines = csv.split("\r\n");
        assert_eq!(
            lines.next().unwrap(),
            "Time,Event,Application,PID,Protocol,Local address,Local port,Remote address,Remote port,State,Previous state"
        );
        let row = lines.next().unwrap();
        assert!(row.contains("Port started listening"));
        assert!(row.contains("\"my,app.exe\""));
        assert!(row.contains(",8080,"));
    }
}
