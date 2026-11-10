import { Pause, Play, RefreshCw } from "lucide-react";
import { useState, type Ref } from "react";
import { api, errorMessage } from "../lib/api";
import { useSnapshot, useUI, type Page } from "../state/AppContext";
import { IconButton, SearchBox } from "./Controls";

const TITLES: Record<Page, [string, string]> = {
  overview: ["Overview", "A live summary of network activity on this computer"],
  connections: ["Connections", "Every TCP connection and UDP endpoint Windows reports"],
  ports: ["Ports", "Which local ports are open and which program owns them"],
  applications: ["Applications", "Programs observed using the network while PortWatch was running"],
  activity: ["Activity", "What changed recently, grouped by application"],
  history: ["History", "Changes recorded on this computer over time"],
  alerts: ["Alerts", "Notifications PortWatch has raised"],
  settings: ["Settings", "Monitoring, notifications, history and appearance"],
  privacy: ["Privacy", "What PortWatch does and does not do with your data"],
};

export const SEARCHABLE: Page[] = ["connections", "ports", "applications", "activity", "history", "alerts"];

export function Header({ searchRef }: { searchRef: Ref<HTMLInputElement> }) {
  const { page, search, setSearch, setPage, toast } = useUI();
  const { snapshot } = useSnapshot();
  const [busy, setBusy] = useState(false);
  const [title, subtitle] = TITLES[page];
  const paused = snapshot?.paused ?? false;
  const live = snapshot?.hasData && !snapshot.error && !paused;

  const togglePause = async () => {
    try { await api.setPaused(!paused); } catch (e) { toast(errorMessage(e), "error"); }
  };
  const refresh = async () => {
    setBusy(true);
    try { await api.refreshNow(); } catch (e) { toast(errorMessage(e), "error"); }
    window.setTimeout(() => setBusy(false), 600);
  };

  return (
    <header className="header">
      <div className="header-title">
        <h1>{title}</h1>
        <p>{subtitle}</p>
      </div>
      <div className="header-tools">
        {page === "overview" ? (
          <SearchBox
            inputRef={searchRef} value={search} placeholder="Search everything… (Ctrl+K)"
            onChange={(v) => { setSearch(v); if (v.trim()) setPage("connections"); }}
          />
        ) : SEARCHABLE.includes(page) ? (
          <SearchBox inputRef={searchRef} value={search} onChange={setSearch} placeholder="Search app, PID, address, port… (Ctrl+K)" />
        ) : null}
        <span className={`status-pill ${paused ? "paused" : live ? "live" : "idle"}`} role="status">
          <i className="dot" />
          {paused ? "Paused" : snapshot?.error ? "Error" : live ? `Live · every ${(snapshot!.intervalMs / 1000).toFixed(snapshot!.intervalMs % 1000 ? 1 : 0)}s` : "Starting…"}
        </span>
        <IconButton label={paused ? "Resume monitoring" : "Pause monitoring"} onClick={togglePause}>
          {paused ? <Play size={16} /> : <Pause size={16} />}
        </IconButton>
        <IconButton label="Refresh now" onClick={refresh} disabled={paused || busy}>
          <RefreshCw size={16} className={busy ? "spin" : ""} />
        </IconButton>
      </div>
    </header>
  );
}
