import { Activity, AppWindow, Bell, History, LayoutDashboard, Network, PlugZap, Settings, ShieldCheck } from "lucide-react";
import type { ReactNode } from "react";
import { Logo } from "./Logo";
import { useSnapshot, useUI, type Page } from "../state/AppContext";

export const NAV: { page: Page; label: string; icon: ReactNode }[] = [
  { page: "overview", label: "Overview", icon: <LayoutDashboard size={17} /> },
  { page: "connections", label: "Connections", icon: <Network size={17} /> },
  { page: "ports", label: "Ports", icon: <PlugZap size={17} /> },
  { page: "applications", label: "Applications", icon: <AppWindow size={17} /> },
  { page: "activity", label: "Activity", icon: <Activity size={17} /> },
  { page: "history", label: "History", icon: <History size={17} /> },
  { page: "alerts", label: "Alerts", icon: <Bell size={17} /> },
  { page: "settings", label: "Settings", icon: <Settings size={17} /> },
];

export function Sidebar() {
  const { page, setPage } = useUI();
  const { snapshot } = useSnapshot();
  const unread = snapshot?.unreadAlerts ?? 0;

  return (
    <nav className="sidebar" aria-label="Main navigation">
      <div className="brand">
        <Logo size={28} />
        <div className="brand-text">
          <strong>PortWatch</strong>
          <span>Know what’s happening on your network.</span>
        </div>
      </div>
      <ul className="nav-list">
        {NAV.map((n, i) => (
          <li key={n.page}>
            <button
              type="button" className={`nav-item ${page === n.page ? "active" : ""}`}
              aria-current={page === n.page ? "page" : undefined} title={`${n.label} (Ctrl+${i + 1})`}
              onClick={() => setPage(n.page)}
            >
              {n.icon}
              <span className="nav-label">{n.label}</span>
              {n.page === "alerts" && unread > 0 && <span className="nav-badge" aria-label={`${unread} unread`}>{unread > 99 ? "99+" : unread}</span>}
            </button>
          </li>
        ))}
      </ul>
      <div className="nav-foot">
        <button
          type="button" className={`nav-item ${page === "privacy" ? "active" : ""}`}
          aria-current={page === "privacy" ? "page" : undefined} onClick={() => setPage("privacy")}
        >
          <ShieldCheck size={17} />
          <span className="nav-label">Privacy</span>
        </button>
      </div>
    </nav>
  );
}
