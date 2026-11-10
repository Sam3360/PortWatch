//! Process information (names, executable paths, start times).
//!
//! On Windows this uses the Toolhelp snapshot API for names (works without
//! elevation for every process) and `OpenProcess` with
//! `PROCESS_QUERY_LIMITED_INFORMATION` for paths and start times. Protected
//! processes deny that access, in which case the fields are reported as
//! unavailable rather than guessed.

use std::collections::HashMap;
use std::path::Path;

#[cfg(windows)]
mod win;

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessQuery {
    pub path: Option<String>,
    /// Unix milliseconds.
    pub start_time: Option<i64>,
    /// False when the PID no longer exists (or is the pseudo PID 0).
    pub running: bool,
    /// True when Windows let us open the process for inspection.
    pub accessible: bool,
}

/// Converts a Windows FILETIME (100 ns ticks since 1601-01-01) to Unix milliseconds.
pub fn filetime_to_unix_ms(low: u32, high: u32) -> Option<i64> {
    const EPOCH_DIFF_100NS: u64 = 116_444_736_000_000_000;
    let ticks = ((high as u64) << 32) | low as u64;
    if ticks < EPOCH_DIFF_100NS {
        return None;
    }
    Some(((ticks - EPOCH_DIFF_100NS) / 10_000) as i64)
}

/// PID -> executable file name for every running process.
#[cfg(windows)]
pub fn process_names() -> HashMap<u32, String> {
    win::process_names()
}

#[cfg(not(windows))]
pub fn process_names() -> HashMap<u32, String> {
    HashMap::new()
}

#[cfg(windows)]
pub fn query(pid: u32) -> ProcessQuery {
    win::query(pid)
}

#[cfg(not(windows))]
pub fn query(_pid: u32) -> ProcessQuery {
    ProcessQuery {
        path: None,
        start_time: None,
        running: false,
        accessible: false,
    }
}

/// Opens Explorer with the file selected. The path must come from [`query`],
/// never from the frontend, and must exist.
#[cfg(windows)]
pub fn reveal_in_explorer(path: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    if !Path::new(path).is_file() {
        return Err("The executable no longer exists at that location.".into());
    }
    // Windows paths cannot contain double quotes, so quoting here is safe.
    std::process::Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{path}\""))
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open Explorer: {e}"))
}

#[cfg(not(windows))]
pub fn reveal_in_explorer(_path: &str) -> Result<(), String> {
    Err("Opening folders is only supported on Windows.".into())
}

/// Opens a folder that PortWatch itself chose (its data directory).
#[cfg(windows)]
pub fn open_folder(dir: &Path) -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open Explorer: {e}"))
}

#[cfg(not(windows))]
pub fn open_folder(_dir: &Path) -> Result<(), String> {
    Err("Opening folders is only supported on Windows.".into())
}

/// Opens a fixed https URL in the default browser. Callers pass a constant only.
#[cfg(windows)]
pub fn open_url(url: &'static str) -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open the browser: {e}"))
}

#[cfg(not(windows))]
pub fn open_url(_url: &'static str) -> Result<(), String> {
    Err("Opening links is only supported on Windows.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filetime_conversion() {
        // 2021-01-01T00:00:00Z
        let ticks: u64 = 132_539_328_000_000_000;
        let ms = filetime_to_unix_ms((ticks & 0xFFFF_FFFF) as u32, (ticks >> 32) as u32);
        assert_eq!(ms, Some(1_609_459_200_000));
    }

    #[test]
    fn filetime_before_unix_epoch_is_none() {
        assert_eq!(filetime_to_unix_ms(0, 0), None);
        assert_eq!(filetime_to_unix_ms(1, 0), None);
    }
}
