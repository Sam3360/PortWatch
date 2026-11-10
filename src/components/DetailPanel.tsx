import { useEffect, useState, type ReactNode } from "react";
import { FolderOpen, Info, X } from "lucide-react";
import { api, errorMessage } from "../lib/api";
import { describeEvent, eventLabel } from "../lib/events";
import {
  bindingScope, formatDateTime, formatDuration, formatEndpoint, processLabel,
} from "../lib/format";
import type { Connection, ProcessDetails } from "../lib/types";
import { useSnapshot, useUI } from "../state/AppContext";
import { Button } from "./Controls";
import { Banner, Spinner } from "./Feedback";
import { ProtocolBadge, StateBadge } from "./Badges";

function KV({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="kv">
      <dt>{label}</dt>
      <dd>{children}</dd>
    </div>
  );
}

function useProcessDetails(pid: number | null) {
  const [details, setDetails] = useState<ProcessDetails | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  useEffect(() => {
    if (pid == null) return;
    let alive = true;
    setLoading(true);
    setError(null);
    api.processDetails(pid)
      .then((d) => alive && setDetails(d))
      .catch((e) => alive && setError(errorMessage(e)))
      .finally(() => alive && setLoading(false));
    return () => { alive = false; };
  }, [pid]);
  return { details: pid == null ? null : details, error, loading };
}

function ProcessSection({ pid, fallbackName }: { pid: number; fallbackName: string | null }) {
  const { details, error, loading } = useProcessDetails(pid);
  const { toast } = useUI();
  const open = async () => {
    try { await api.revealProcessFolder(pid); } catch (e) { toast(errorMessage(e), "error"); }
  };
  if (loading && !details) return <Spinner label="Inspecting process…" />;
  if (error) return <Banner tone="error">{error}</Banner>;
  return (
    <>
      <dl className="kvs">
        <KV label="Process name">{processLabel(details?.name ?? fallbackName, pid)}</KV>
        <KV label="PID">{pid === 0 ? "None reported" : pid}</KV>
        <KV label="Executable path"><span className="mono wrap">{details?.path ?? "Unavailable"}</span></KV>
        <KV label="Process started">{formatDateTime(details?.startTime ?? null)}</KV>
      </dl>
      {details?.note && <Banner tone="info">{details.note}</Banner>}
      <div className="panel-actions">
        <Button icon={<FolderOpen size={15} />} disabled={!details?.path} onClick={open}
          title={details?.path ? "Show the executable in File Explorer" : "The executable path is unavailable"}>
          Open containing folder
        </Button>
      </div>
    </>
  );
}

function ConnectionPanel({ c, from }: { c: Connection; from: "connections" | "ports" }) {
  const { snapshot } = useSnapshot();
  const now = snapshot?.timestamp ?? Date.now();
  const current = snapshot?.connections.find((x) => x.id === c.id);
  const shown = current ?? c;
  return (
    <>
      {!current && <Banner tone="warn">This connection is no longer present. Showing the last details PortWatch saw.</Banner>}
      <section>
        <h3>Process</h3>
        <ProcessSection pid={shown.pid} fallbackName={shown.processName} />
      </section>
      <section>
        <h3>{shown.isListening ? "Port" : "Connection"}</h3>
        <dl className="kvs">
          <KV label="Role">{shown.isListening ? (shown.protocol === "UDP" ? "Bound UDP endpoint (can receive datagrams)" : "Listening for incoming connections") : "Connected"}</KV>
          <KV label="Protocol"><ProtocolBadge protocol={shown.protocol} /> <span className="muted">{shown.family}</span></KV>
          <KV label="State"><StateBadge state={shown.state} /></KV>
          <KV label="Local endpoint"><span className="mono">{formatEndpoint(shown.localAddress, shown.localPort)}</span></KV>
          <KV label="Remote endpoint"><span className="mono">{shown.remoteAddress ? formatEndpoint(shown.remoteAddress, shown.remotePort) : "None (not connected)"}</span></KV>
          {shown.isListening && <KV label="Bound to">{bindingScope(shown.localAddress)}</KV>}
          <KV label="First observed">{formatDateTime(shown.firstSeen)}</KV>
          <KV label="Last observed">{current ? formatDateTime(now) : "Not present in the latest reading"}</KV>
          <KV label="Observed for">{formatDuration(now - shown.firstSeen)}</KV>
        </dl>
        <p className="fine">“First observed” is when PortWatch first saw this socket, not when Windows created it.</p>
      </section>
      {from === "ports" && (
        <section>
          <Banner tone="info">
            A listening port does not mean something is wrong. Many normal programs listen for connections, often only from this computer.
            “Bound to” shows which network interfaces the program accepts traffic on; Windows Firewall rules may still block it.
          </Banner>
        </section>
      )}
    </>
  );
}

function ApplicationPanel({ pid }: { pid: number }) {
  const { snapshot } = useSnapshot();
  const { select } = useUI();
  const app = snapshot?.applications.find((a) => a.pid === pid);
  const conns = snapshot?.connections.filter((c) => c.pid === pid) ?? [];
  if (!app) return <Banner tone="warn">This application is no longer listed.</Banner>;
  return (
    <>
      {!app.active && <Banner tone="warn">No network activity from this process in the latest reading.</Banner>}
      <section>
        <h3>Process</h3>
        <ProcessSection pid={app.pid} fallbackName={app.name} />
      </section>
      <section>
        <h3>Observed activity</h3>
        <dl className="kvs">
          <KV label="First observed">{formatDateTime(app.firstSeen)}</KV>
          <KV label="Last observed">{formatDateTime(app.lastSeen)}</KV>
          <KV label="Connections now">{app.connectionCount}</KV>
          <KV label="Connections seen">{app.observedConnections.toLocaleString()}</KV>
          <KV label="Listening ports"><span className="mono">{app.listeningPorts.join(", ") || "None"}</span></KV>
        </dl>
        <p className="fine">Only activity seen while PortWatch was running is counted.</p>
      </section>
      {conns.length > 0 && (
        <section>
          <h3>Current sockets ({conns.length})</h3>
          <ul className="mini-list">
            {conns.slice(0, 100).map((c) => (
              <li key={c.id}>
                <button type="button" onClick={() => select({ kind: "connection", connection: c, from: "connections" })}>
                  <ProtocolBadge protocol={c.protocol} />
                  <span className="mono">{formatEndpoint(c.localAddress, c.localPort)}{c.remoteAddress ? ` → ${formatEndpoint(c.remoteAddress, c.remotePort)}` : ""}</span>
                </button>
              </li>
            ))}
          </ul>
          {conns.length > 100 && <p className="fine">Showing the first 100.</p>}
        </section>
      )}
    </>
  );
}

export function DetailPanel() {
  const { selection, select } = useUI();
  if (!selection) return null;

  let title = "Details";
  let body: ReactNode = null;
  if (selection.kind === "connection") {
    title = selection.connection.isListening ? `Port ${selection.connection.localPort}` : "Connection";
    body = <ConnectionPanel c={selection.connection} from={selection.from} />;
  } else if (selection.kind === "application") {
    title = "Application";
    body = <ApplicationPanel pid={selection.pid} />;
  } else {
    const e = selection.event;
    title = eventLabel(e.kind);
    body = (
      <>
        <section>
          <dl className="kvs">
            <KV label="When">{formatDateTime(e.ts)}</KV>
            <KV label="Event">{eventLabel(e.kind)}</KV>
            <KV label="Summary">{describeEvent(e)}</KV>
            {e.protocol && <KV label="Protocol"><ProtocolBadge protocol={e.protocol} /></KV>}
            {e.localAddress && <KV label="Local endpoint"><span className="mono">{formatEndpoint(e.localAddress, e.localPort)}</span></KV>}
            {e.remoteAddress && <KV label="Remote endpoint"><span className="mono">{formatEndpoint(e.remoteAddress, e.remotePort)}</span></KV>}
            {e.state && <KV label="State">{e.state}</KV>}
            {e.prevState && <KV label="Previous state">{e.prevState}</KV>}
            <KV label="Process">{processLabel(e.processName, e.pid)}{e.pid ? ` (PID ${e.pid})` : ""}</KV>
          </dl>
          <p className="fine">The process may have exited since this was recorded; PIDs are reused by Windows over time.</p>
        </section>
        {e.pid !== 0 && (
          <section>
            <h3>Process now</h3>
            <ProcessSection pid={e.pid} fallbackName={e.processName} />
          </section>
        )}
      </>
    );
  }

  return (
    <aside className="detail" aria-label="Details panel">
      <div className="detail-head">
        <h2><Info size={16} aria-hidden /> {title}</h2>
        <button type="button" className="icon-btn" aria-label="Close details (Esc)" onClick={() => select(null)}><X size={16} /></button>
      </div>
      <div className="detail-body">{body}</div>
    </aside>
  );
}

