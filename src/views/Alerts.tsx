import { useEffect, useState } from "react";
import { Bell, CheckCheck, Trash2 } from "lucide-react";
import { api, errorMessage } from "../lib/api";
import { formatDateTime, formatRelative } from "../lib/format";
import { matchesSearch } from "../lib/filters";
import type { AlertItem } from "../lib/types";
import { Button, Switch } from "../components/Controls";
import { Banner, EmptyState, Spinner } from "../components/Feedback";
import { useSettings, useSnapshot, useUI } from "../state/AppContext";

const CATEGORY_LABEL: Record<string, string> = {
  new_listener: "New listener",
  new_connection: "New connection",
  monitored_port: "Monitored port",
  process_disappeared: "Process exited",
};

export function Alerts() {
  const { settings, save } = useSettings();
  const { snapshot } = useSnapshot();
  const { search, confirm, toast, setPage } = useUI();
  const [alerts, setAlerts] = useState<AlertItem[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  const unread = snapshot?.unreadAlerts ?? 0;
  const now = snapshot?.timestamp ?? Date.now();

  useEffect(() => {
    let alive = true;
    api.alerts().then((a) => { if (alive) { setAlerts(a); setError(null); } }).catch((e) => alive && setError(errorMessage(e)));
    return () => { alive = false; };
  }, [unread, tick]);

  const markRead = async () => {
    try { await api.markAlertsRead(); setTick((t) => t + 1); } catch (e) { toast(errorMessage(e), "error"); }
  };
  const clear = async () => {
    if (!(await confirm({ title: "Clear notification history?", message: "This deletes the list of past alerts. It does not change which alerts are enabled.", confirmLabel: "Clear alerts", danger: true }))) return;
    try { await api.clearAlerts(); setTick((t) => t + 1); toast("Notification history cleared.", "success"); } catch (e) { toast(errorMessage(e), "error"); }
  };

  const toggle = async (patch: Partial<NonNullable<typeof settings>["notifications"]>) => {
    if (!settings) return;
    try { await save({ ...settings, notifications: { ...settings.notifications, ...patch } }); }
    catch (e) { toast(errorMessage(e), "error"); }
  };

  if (error) return <Banner tone="error">Could not load alerts: {error}</Banner>;
  if (!alerts || !settings) return <Spinner label="Loading alerts…" />;
  const n = settings.notifications;
  const shown = alerts.filter((a) => matchesSearch({ processName: a.processName, pid: a.pid, localAddress: "", localPort: a.port ?? 0, remoteAddress: null, remotePort: null, protocol: a.protocol ?? "" }, search) || (search.trim() !== "" && (a.title + " " + a.message).toLowerCase().includes(search.toLowerCase())));

  return (
    <div className="page">
      <section className="card">
        <div className="card-head">
          <h2>Alert categories</h2>
          <button type="button" className="link-btn" onClick={() => setPage("settings")}>More notification settings</button>
        </div>
        <Banner tone="info">Alerts describe what PortWatch observed. An unfamiliar program is not necessarily harmful, and PortWatch does not judge whether it is.</Banner>
        <Switch label="Notifications enabled" hint="Master switch for all alerts below." checked={n.enabled} onChange={(v) => toggle({ enabled: v })} />
        <Switch label="New listening port" hint="A program that has not been seen listening before starts listening on a TCP port." checked={n.newListener} disabled={!n.enabled} onChange={(v) => toggle({ newListener: v })} />
        <Switch label="Monitored port changes" hint={n.monitoredPorts.length ? `Watching: ${n.monitoredPorts.join(", ")}` : "No ports selected yet. Add some in Settings."} checked={n.monitoredPortChange} disabled={!n.enabled} onChange={(v) => toggle({ monitoredPortChange: v })} />
        <Switch label="New outbound connection" hint="First time a program connects to a given remote address. Can be noisy." checked={n.newConnection} disabled={!n.enabled} onChange={(v) => toggle({ newConnection: v })} />
        <Switch label="Listening program exits" hint="A program previously seen listening is no longer running." checked={n.processDisappeared} disabled={!n.enabled} onChange={(v) => toggle({ processDisappeared: v })} />
      </section>

      <section className="card">
        <div className="card-head">
          <h2>Notification history</h2>
          <div className="row-gap">
            <Button icon={<CheckCheck size={15} />} onClick={markRead} disabled={unread === 0}>Mark all read</Button>
            <Button variant="danger" icon={<Trash2 size={15} />} onClick={clear} disabled={alerts.length === 0}>Clear</Button>
          </div>
        </div>
        {alerts.length === 0 ? (
          <EmptyState icon={<Bell size={28} />} title="No alerts yet">
            Alerts appear here when PortWatch observes something you asked to be told about. Programs already listening when PortWatch starts do not trigger alerts.
          </EmptyState>
        ) : shown.length === 0 ? (
          <p className="muted">No alerts match your search.</p>
        ) : (
          <ul className="alert-list">
            {shown.map((a) => (
              <li key={a.id} className={a.read ? "" : "unread"}>
                <div className="alert-head">
                  <strong>{a.title}</strong>
                  <span className="badge state-muted">{CATEGORY_LABEL[a.category] ?? a.category}</span>
                  <span className="muted alert-time" title={formatDateTime(a.ts)}>{formatRelative(a.ts, now)}</span>
                </div>
                <p>{a.message}</p>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
