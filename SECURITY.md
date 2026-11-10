# Security Policy

## Reporting a vulnerability

Please **don't open a public issue** for security problems. Use GitHub's private vulnerability reporting on the repository (Security → Report a vulnerability), or contact the maintainer through the email on the GitHub profile [@Sam3360](https://github.com/Sam3360).

Include what you found, how to reproduce it, and the PortWatch version. I'll acknowledge reports as soon as I can; this is a one-person hobby project, so there is no guaranteed response time.

## Supported versions

Only the latest release receives fixes.

## Design notes relevant to security

- The UI runs in a Tauri web view with a strict content-security policy (`connect-src` limited to Tauri IPC). The capability file grants only `core:default`.
- All system access goes through a fixed set of Rust commands. The frontend can't supply file paths to open or execute: "open containing folder" takes a PID and the backend looks the path up itself; CSV export writes only to a path chosen in the native Save dialog.
- No shell commands are built from network or process data. The only child process is `explorer.exe`, launched with a path from Windows' own process query, the data folder, or a fixed URL.
- SQL uses parameterised queries; search text is escaped for `LIKE`.
- CSV output neutralises leading `= + - @` so exported process names can't run as spreadsheet formulas.
- PortWatch makes no network requests.

## Scope

PortWatch reports network state. It isn't a security product and makes no claim to detect or prevent attacks.
