# Changelog

All notable changes are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/) and the project uses [Semantic Versioning](https://semver.org/).

## [1.0.0] - Unreleased

First version of the source. No binaries have been published yet.

### Added
- Live monitoring of TCP and UDP sockets (IPv4 and IPv6) using the Windows IP Helper API, with owning process, executable path and start time where Windows allows it.
- Overview, Connections, Ports, Applications, Activity, History, Alerts, Settings and Privacy pages.
- Local SQLite history with search, date/protocol/application/event filters, retention settings, CSV export and clear.
- Neutral, de-duplicated notifications with a notification history.
- CSV export for connections, ports, applications and history.
- System tray (show, hide, status, pause/resume, exit), close-to-tray option, opt-in launch at startup, start minimized.
- Dark (default), light and system themes.
- Portable mode (marker file) and NSIS installer configuration.
- Rust unit tests (table parsing, diffing, history, settings, alerts, CSV) and Vitest tests for the frontend logic.
