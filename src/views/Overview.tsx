import { useEffect, useMemo, useState } from "react";
import { AppWindow, ArrowRightLeft, Network, PlugZap, Radio, Waypoints } from "lucide-react";
import { api } from "../lib/api";
import { describeEvent } from "../lib/events";
import { formatClock, formatDuration } from "../lib/format";
import { computeStats, topApplications } from "../lib/stats";
import type { HistoryEvent } from "../lib/types";
import { EventBadge } from "../components/Badges";
import { Banner, EmptyState, Spinner } from "../components/Feedback";
import { Sparkline, StatCard } from "../components/StatCard";
import { useSnapshot, useUI } from "../state/AppContext";

export function Overview() {
  const { snapshot, loadError, trend } = useSnapshot();
  const { setPage, select } = useUI();
  const [recent, setRecent] = useState<HistoryEvent[]>([]);
  const seq = snapshot?.eventSeq ?? 0;

  useEffect(() => {
    let alive = true;
    api.recentEvents(8).then((e) => alive && setRecent(e)).catch(() => {});
    return () => { alive = false; };
  }, [seq]);

  const stats = useMemo(() => (snapshot ? computeStats(snapshot.connections, snapshot.applications) : null), [snapshot]);
  const top = useMemo(() => (snapshot ? topApplications(snapshot.applications, 6) : []), [snapshot]);

  if (loadError) return <Banner tone="error">Could not read monitoring data: {loadError}</Banner>;
  if (!snapshot || !stats) return <Spinner label="Reading network state…" />;

  const maxTop = Math.max(1, ...top.map((t) => t.count));
  return (
    <div className="page">
      {!snapshot.platformSupported && (
        <Banner tone="error">PortWatch reads Windows networking APIs and can only monitor on Windows 10/11. No data is shown on this platform.</Banner>
      )}
      {snapshot.error && <Banner tone="error">{snapshot.error}</Banner>}
      {snapshot.paused && <Banner tone="warn">Monitoring is paused. The numbers below are from the last reading. Changes made while paused are not recorded.</Banner>}

      <div className="stat-grid">
        <StatCard label="Active connections" icon={<ArrowRightLeft size={16} />} value={stats.active.toLocaleString()} sub={`${stats.total.toLocaleString()} sockets in total`} />
        <StatCard label="TCP sockets" icon={<Network size={16} />} value={stats.tcp.toLocaleString()} sub="connected, listening and closing" />
        <StatCard label="UDP endpoints" icon={<Radio size={16} />} value={stats.udp.toLocaleString()} sub="bound ports (UDP has no connections)" />
        <StatCard label="Listening ports" icon={<PlugZap size={16} />} value={stats.listeningTcp.toLocaleString()} sub="TCP ports accepting connections" />
        <StatCard label="Applications" icon={<AppWindow size={16} />} value={stats.uniqueApplications.toLocaleString()} sub="with open sockets right now" />
      </div>

      {snapshot.hasData && snapshot.connections.length === 0 && !snapshot.error ? (
        <EmptyState icon={<Waypoints size={28} />} title="No sockets reported">
          Windows returned no TCP or UDP sockets in the latest reading.
        </EmptyState>
      ) : (
        <div className="grid-2">
          <section className="card">
            <div className="card-head">
              <h2>Active connections over time</h2>
              <span className="muted">Since PortWatch started · {formatDuration(snapshot.timestamp - snapshot.monitoringSince)}</span>
            </div>
            <Sparkline values={trend.map((t) => t.active)} />
          </section>
          <section className="card">
            <div className="card-head">
              <h2>Busiest applications</h2>
              <button type="button" className="link-btn" onClick={() => setPage("applications")}>All applications</button>
            </div>
            {top.length === 0 ? <p className="muted">No applications have open sockets.</p> : (
              <ul className="bars">
                {top.map((t) => (
                  <li key={t.name}>
                    <span className="bar-name" title={t.name}>{t.name}</span>
                    <span className="bar-track"><span className="bar-fill" style={{ width: `${(t.count / maxTop) * 100}%` }} /></span>
                    <span className="bar-count">{t.count}</span>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      )}

      <section className="card">
        <div className="card-head">
          <h2>Recent changes</h2>
          <button type="button" className="link-btn" onClick={() => setPage("activity")}>Open activity</button>
        </div>
        {recent.length === 0 ? (
          <p className="muted">
            No changes have been detected since PortWatch started. Connections that already existed at startup are not listed as changes.
          </p>
        ) : (
          <ul className="recent">
            {recent.map((e) => (
              <li key={e.id}>
                <button type="button" onClick={() => select({ kind: "event", event: e })}>
                  <span className="mono muted">{formatClock(e.ts)}</span>
                  <EventBadge kind={e.kind} />
                  <span className="truncate">{describeEvent(e)}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
