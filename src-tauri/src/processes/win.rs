//! Windows implementation of the process helpers. Unsafe FFI is confined here.

use std::collections::HashMap;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_INVALID_PARAMETER, FILETIME, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};

use super::{filetime_to_unix_ms, ProcessQuery};

/// Closes the handle when dropped so no path out of `query` can leak it.
struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: the handle was returned by a successful Win32 call and is closed once.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub fn process_names() -> HashMap<u32, String> {
    let mut map = HashMap::new();
    // SAFETY: standard Toolhelp enumeration; the snapshot handle is closed below.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return map;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                map.insert(entry.th32ProcessID, name);
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
    }
    map
}

pub fn query(pid: u32) -> ProcessQuery {
    if pid == 0 {
        // PID 0 is the idle pseudo-process; Windows uses it when it has no owner to report.
        return ProcessQuery {
            path: None,
            start_time: None,
            running: false,
            accessible: false,
        };
    }
    // SAFETY: plain Win32 calls with valid out pointers; the handle is wrapped for cleanup.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            let err = GetLastError();
            // ERROR_INVALID_PARAMETER means the PID does not exist (any more).
            // Access denied and other errors mean it exists but cannot be inspected.
            return ProcessQuery {
                path: None,
                start_time: None,
                running: err != ERROR_INVALID_PARAMETER,
                accessible: false,
            };
        }
        let guard = OwnedHandle(handle);

        let mut creation: FILETIME = std::mem::zeroed();
        let mut exit: FILETIME = std::mem::zeroed();
        let mut kernel: FILETIME = std::mem::zeroed();
        let mut user: FILETIME = std::mem::zeroed();
        let mut start_time = None;
        let mut running = true;
        if GetProcessTimes(guard.0, &mut creation, &mut exit, &mut kernel, &mut user) != 0 {
            start_time = filetime_to_unix_ms(creation.dwLowDateTime, creation.dwHighDateTime);
            if exit.dwLowDateTime != 0 || exit.dwHighDateTime != 0 {
                running = false;
            }
        }

        let mut buf = vec![0u16; 32_768];
        let mut len = buf.len() as u32;
        let path = if QueryFullProcessImageNameW(guard.0, 0, buf.as_mut_ptr(), &mut len) != 0 {
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        } else {
            None
        };

        ProcessQuery {
            path,
            start_time,
            running,
            accessible: true,
        }
    }
}
