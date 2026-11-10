# Contributing to PortWatch

Thanks for taking a look. This is a small personal project, so keep changes focused.

## Ground rules

- PortWatch is local-first. Please don't add telemetry, analytics, accounts, update pings, cloud services or any outbound network request.
- Don't add fake or sample data. Empty states should be honest.
- Wording about processes and ports stays neutral. Don't call something "suspicious" or "malicious"; report what was observed.
- Keep dependencies minimal and justify new ones in the pull request.
- Keep `unsafe` code inside the two small Windows FFI modules (`network/winapi.rs`, `processes/win.rs`), with a `// SAFETY:` comment.
- Frontend never passes file paths to the backend to open or run. Paths come from the backend's own process lookup.

## Setup

See the README ("Development setup"). In short: Rust (MSVC), Node.js 18+, `npm install`, `npm run tauri:dev`.

## Before opening a pull request

```powershell
npm run typecheck
npm test
npm run test:rust
npm run fmt:rust
```

Add tests for new logic. Rust tests must not depend on your live connections: build fixtures (see `network/parse.rs` → `fixtures`) or use the in-memory database.

## Layout

```
src/                 React + TypeScript UI (views/, components/, lib/, state/)
src-tauri/src/
  network/           IP Helper calls, table parsing, diffing, monitor loop
  processes/         names, paths, start times
  history/           SQLite storage
  notifications/     alert rules
  settings/          validated settings
  tray/              system tray
  commands/          the API exposed to the UI
```

## Commit style

Short, imperative subject lines ("Fix TIME_WAIT owner lookup"). One logical change per commit where practical.
