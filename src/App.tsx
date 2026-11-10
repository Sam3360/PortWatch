import { useEffect, useRef } from "react";
import { isTauri } from "./lib/api";
import { ConfirmDialog, EmptyState, ToastHost } from "./components/Feedback";
import { DetailPanel } from "./components/DetailPanel";
import { Header } from "./components/Header";
import { NAV, Sidebar } from "./components/Sidebar";
import { AppProviders, useUI, type Page } from "./state/AppContext";
import { Activity } from "./views/Activity";
import { Alerts } from "./views/Alerts";
import { Applications } from "./views/Applications";
import { Connections } from "./views/Connections";
import { History } from "./views/History";
import { Overview } from "./views/Overview";
import { Ports } from "./views/Ports";
import { Privacy } from "./views/Privacy";
import { Settings } from "./views/Settings";

const VIEWS: Record<Page, () => JSX.Element> = {
  overview: Overview, connections: Connections, ports: Ports, applications: Applications,
  activity: Activity, history: History, alerts: Alerts, settings: Settings, privacy: Privacy,
};

function Shell() {
  const { page, setPage, selection, select } = useUI();
  const searchRef = useRef<HTMLInputElement>(null);
  const View = VIEWS[page];

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const typing = target && (target.tagName === "INPUT" || target.tagName === "SELECT" || target.tagName === "TEXTAREA");
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        searchRef.current?.focus();
        searchRef.current?.select();
      } else if ((e.ctrlKey || e.metaKey) && /^[1-8]$/.test(e.key)) {
        e.preventDefault();
        setPage(NAV[Number(e.key) - 1].page);
      } else if (e.key === "/" && !typing) {
        e.preventDefault();
        searchRef.current?.focus();
      } else if (e.key === "Escape" && selection && !document.querySelector("dialog[open]")) {
        select(null);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selection, select, setPage]);

  return (
    <div className={`app ${selection ? "with-detail" : ""}`}>
      <Sidebar />
      <div className="main">
        <Header searchRef={searchRef} />
        <main className="content" id="content" key={page}>
          <View />
        </main>
      </div>
      <DetailPanel />
      <ToastHost />
      <ConfirmDialog />
    </div>
  );
}

export default function App() {
  if (!isTauri()) {
    return (
      <div className="standalone">
        <EmptyState title="PortWatch needs its desktop shell">
          This page is the PortWatch interface, which reads live Windows network data through its Rust backend. It does not work in a plain browser.
          Run <code>npm run tauri:dev</code> to start the app.
        </EmptyState>
      </div>
    );
  }
  return (
    <AppProviders>
      <Shell />
    </AppProviders>
  );
}
