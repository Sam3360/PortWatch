//! Windows-only: calls the IP Helper API and returns the raw table buffers.
//! This is the only place in the network layer that uses `unsafe`.

use std::ffi::c_void;

use windows_sys::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
use windows_sys::Win32::NetworkManagement::IpHelper::{GetExtendedTcpTable, GetExtendedUdpTable};

use super::parse::{self, ParseError, RawSocket};
use super::Enumeration;

const AF_INET: u32 = 2;
const AF_INET6: u32 = 23;
/// TCP_TABLE_OWNER_PID_ALL: every TCP socket, including listeners, with owning PID.
const TCP_TABLE_OWNER_PID_ALL: i32 = 5;
/// UDP_TABLE_OWNER_PID: every UDP endpoint with owning PID.
const UDP_TABLE_OWNER_PID: i32 = 1;

#[derive(Clone, Copy)]
enum Table {
    Tcp,
    Udp,
}

/// Calls the API in a retry loop: the table can grow between the size query and
/// the data call, in which case Windows returns `ERROR_INSUFFICIENT_BUFFER` again.
fn fetch(table: Table, family: u32) -> Result<Vec<u8>, String> {
    let mut size: u32 = 0;
    let mut buf: Vec<u8> = Vec::new();
    for _ in 0..8 {
        let ptr = if buf.is_empty() {
            std::ptr::null_mut()
        } else {
            buf.as_mut_ptr() as *mut c_void
        };
        // SAFETY: `ptr` is either null (size query) or points to `buf`, which is
        // at least `size` bytes long. `size` is a valid out pointer.
        let status = unsafe {
            match table {
                Table::Tcp => GetExtendedTcpTable(ptr, &mut size, 0, family, TCP_TABLE_OWNER_PID_ALL, 0),
                Table::Udp => GetExtendedUdpTable(ptr, &mut size, 0, family, UDP_TABLE_OWNER_PID, 0),
            }
        };
        if status == NO_ERROR {
            buf.truncate(size as usize);
            return Ok(buf);
        }
        if status == ERROR_INSUFFICIENT_BUFFER {
            // Leave headroom for sockets created in the meantime.
            buf = vec![0u8; size as usize + 8192];
            continue;
        }
        return Err(format!("Windows returned error code {status}"));
    }
    Err("the connection table kept changing size".into())
}

fn collect(
    label: &str,
    bytes: Result<Vec<u8>, String>,
    parser: fn(&[u8]) -> Result<Vec<RawSocket>, ParseError>,
    sockets: &mut Vec<RawSocket>,
    warnings: &mut Vec<String>,
    ok: &mut u32,
) {
    match bytes.and_then(|b| parser(&b).map_err(|e| e.to_string())) {
        Ok(mut rows) => {
            sockets.append(&mut rows);
            *ok += 1;
        }
        Err(e) => warnings.push(format!("{label} could not be read: {e}")),
    }
}

pub fn enumerate() -> Result<Enumeration, String> {
    let mut sockets = Vec::new();
    let mut warnings = Vec::new();
    let mut ok = 0u32;
    collect(
        "TCP (IPv4)",
        fetch(Table::Tcp, AF_INET),
        parse::parse_tcp4,
        &mut sockets,
        &mut warnings,
        &mut ok,
    );
    collect(
        "TCP (IPv6)",
        fetch(Table::Tcp, AF_INET6),
        parse::parse_tcp6,
        &mut sockets,
        &mut warnings,
        &mut ok,
    );
    collect(
        "UDP (IPv4)",
        fetch(Table::Udp, AF_INET),
        parse::parse_udp4,
        &mut sockets,
        &mut warnings,
        &mut ok,
    );
    collect(
        "UDP (IPv6)",
        fetch(Table::Udp, AF_INET6),
        parse::parse_udp6,
        &mut sockets,
        &mut warnings,
        &mut ok,
    );
    if ok == 0 {
        return Err(format!(
            "Could not read any network tables. {}",
            warnings.join("; ")
        ));
    }
    Ok(Enumeration { sockets, warnings })
}
