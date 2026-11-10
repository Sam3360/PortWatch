import { useCallback, useEffect, useMemo, useState } from "react";
import { Database, Download, FilterX, History as HistoryIcon, Trash2 } from "lucide-react";
import { api, errorMessage } from "../lib/api";
import { describeEvent, EVENT_LABELS, eventTone } from "../lib/events";
import { formatClock, formatDate } from "../lib/format";
import { useDebounced } from "../lib/hooks";
import type { HistoryEvent, HistoryFilter, StorageInfo } from "../lib/types";
import { Button, Select, TextField, Toolbar } from "../components/Controls";
import { Banner, EmptyState, Spinner } from "../components/Feedback";
import { useSettings, useSnapshot, useUI } from "../state/AppContext";

const PAGE = 200;

const startOfDay = (s: string) => (s ? new Date(`${s}T00:00:00`).getTime() : undefined);
const endOfDay = (s: string) => (s ? new Date(`${s}T23:59:59.999`).getTime() : undefined);

export function History() {
  const { settings } = useSettings();
  const { snapshot } = useSnapshot();
  const { search, setSearch, select, confirm, toast, setPage } = useUI();
  const [protocol, setProtocol] = useState("all");
  const [processName, setProcessName] = useState("all");
  const [kind, setKind] = useState("all");
  const [fromDate, setFromDate] = useState("");
  const [toDate, setToDate] = useState("");
  const [limit, setLimit] = useState(PAGE);
  const [events, setEvents] = useState<HistoryEvent[] | null>(null);
  const [total, setTotal] = useState(0);
  const [processes, setProcesses] = useState<string[]>([]);
  const [info, setInfo] = useState<StorageInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  const debouncedSearch = useDebounced(search, 250);
  const refreshKey = useDebounced(snapshot?.eventSeq ?? 0, 1500);

  const filter = useMemo<HistoryFilter>(() => ({
    search: debouncedSearch.trim() || undefined,
    protocol: protocol === "all" ? undefined : protocol,
    process: processName === "all" ? undefined : processName,
    kind: kind === "all" ? undefined : kind,
    from: startOfDay(fromDate),
    to: endOfDay(toDate),
  }), [debouncedSearch, protocol, processName, kind, fromDate, toDate]);

  useEffect(() => { setLimit(PAGE); }, [filter]);

  useEffect(() => {
    let alive = true;
    api.queryHistory({ ...filter, limit, offset: 0 })
      .then((p) => { if (alive) { setEvents(p.events); setTotal(p.total); setError(null); } })
      .catch((e) => alive && setError(errorMessage(e)));
    return () => { alive = false; };
  }, [filter, limit, refreshKey]);

  useEffect(() => {
    let alive = true;
    api.historyProcesses().then((p) => alive && setProcesses(p)).catch(() => {});
    api.storageInfo().then((i) => alive && setInfo(i)).catch(() => {});
    return () => { alive = false; };
  }, [refreshKey]);

  const filtersActive = Object.values(filter).some((v) => v !== undefined);
  const reset = () => { setProtocol("all"); setProcessName("all"); setKind("all"); setFromDate(""); setToDate(""); setSearch(""); };

  const doExport = useCallback(async () => {
    try {
      const path = await api.exportHistory(filter);
      if (path) toast(`History exported to ${path}`, "success");
    } catch (e) { toast(`Export failed: ${errorMessage(e)}`, "error"); }
  }, [filter, toast]);

  const doClear = async () => {
    const ok = await confirm({
      title: "Clear all connection history?",
      message: "This permanently deletes every recorded event from this computer. Alerts and settings are not affected. This cannot be undone.",
      confirmLabel: "Clear history", danger: true,
    });
    if (!ok) return;
    try { await api.clearHistory(); setEvents([]); setTotal(0); setProcesses([]); toast("History cleared.", "success"); }
    catch (e) { toast(errorMessage(e), "error"); }
  };

  // Group by local calendar day for the timeline.
  const groups = useMemo(() => {
    const out: { day: string; items: HistoryEvent[] }[] = [];
    for (const e of events ?? []) {
      const day = formatDate(e.ts);
      if (out.length === 0 || out[out.length - 1].day !== day) out.push({ day, items: [] });
      out[out.length - 1].items.push(e);
    }
    return out;
  }, [events]);

  if (error) return <Banner tone="error">Could not read history: {error}</Banner>;
  if (!events) return <Spinner label="Loading history…" />;

  return (
    <div className="page page-fill">
      {settings && !settings.history.enabled && (
        <Banner tone="warn">History recording is turned off, so no new events are saved. <button type="button" className="link-btn" onClick={() => setPage("settings")}>Change in Settings</button></Banner>
      )}
      <Toolbar>
        <TextField label="From" type="date" value={fromDate} onChange={setFromDate} />
        <TextField label="To" type="date" value={toDate} onChange={setToDate} />
        <Select label="Protocol" value={protocol} onChange={setProtocol} options={[{ value: "all", label: "All" }, { value: "TCP", label: "TCP" }, { value: "UDP", label: "UDP" }]} />
        <Select label="Application" value={processName} onChange={setProcessName} options={[{ value: "all", label: "All applications" }, ...processes.map((p) => ({ value: p, label: p }))]} />
        <Select label="Event" value={kind} onChange={setKind} options={[{ value: "all", label: "All events" }, ...Object.entries(EVENT_LABELS).map(([v, l]) => ({ value: v, label: l }))]} />
        <div className="toolbar-spacer" />
        <Button icon={<FilterX size={15} />} onClick={reset} disabled={!filtersActive}>Reset filters</Button>
        <Button icon={<Download size={15} />} onClick={doExport} disabled={total === 0}>Export CSV</Button>
        <Button variant="danger" icon={<Trash2 size={15} />} onClick={doClear} disabled={!info || info.eventCount === 0}>Clear history</Button>
      </Toolbar>

      <div className="history-scroll">
        {events.length === 0 ? (
          <EmptyState icon={<HistoryIcon size={28} />} title={filtersActive ? "No history matches these filters" : "No history recorded yet"}
            action={filtersActive ? <Button onClick={reset}>Reset filters</Button> : undefined}>
            {filtersActive ? "Try a wider date range or clear the filters." : "Changes are recorded as PortWatch observes them. Nothing is recorded for connections that were already open when it started."}
          </EmptyState>
        ) : (
          <div className="timeline">
            {groups.map((g) => (
              <section key={g.day}>
                <h3 className="tl-day">{g.day}</h3>
                <ul>
                  {g.items.map((e) => (
                    <li key={e.id}>
                      <button type="button" className={`tl-item tone-${eventTone(e.kind)}`} onClick={() => select({ kind: "event", event: e })}>
                        <span className="tl-time mono">{formatClock(e.ts)}</span>
                        <span className="tl-dot" aria-hidden />
                        <span className="tl-main">
                          <strong>{EVENT_LABELS[e.kind] ?? e.kind}</strong>
                          <span className="mono truncate">{describeEvent(e)}</span>
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              </section>
            ))}
            {events.length < total && (
              <div className="load-more">
                <Button onClick={() => setLimit((l) => l + PAGE)}>Load more ({(total - events.length).toLocaleString()} older)</Button>
              </div>
            )}
          </div>
        )}
      </div>

      <div className="storage-note">
        <Database size={14} aria-hidden />
        <span>
          Showing {events.length.toLocaleString()} of {total.toLocaleString()} matching events.
          {info && <> Stored locally at <span className="mono">{info.databasePath}</span>.</>}
          {" "}Use <strong>Clear history</strong> here, or <button type="button" className="link-btn" onClick={() => setPage("settings")}>Settings → Data</button>, to delete it.
        </span>
      </div>
    </div>
  );
}
