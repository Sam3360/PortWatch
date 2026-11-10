import { useEffect, useMemo, useRef, useState } from "react";
import { Activity as ActivityIcon, FilterX } from "lucide-react";
import { api, errorMessage } from "../lib/api";
import { describeEvent } from "../lib/events";
import { type EventTone } from "../lib/events";
import { matchesSearch } from "../lib/filters";
import { formatClock, formatDateTime, processLabel } from "../lib/format";
import type { SortState } from "../lib/sort";
import { buildLanes, pickBucketMs } from "../lib/timeline";
import type { HistoryEvent } from "../lib/types";
import { EventBadge, ProtocolBadge } from "../components/Badges";
import { Button, Select, Toolbar } from "../components/Controls";
import { DataTable, nextSort, useSortedRows, type Column } from "../components/DataTable";
import { Banner, EmptyState, Spinner } from "../components/Feedback";
import { useSnapshot, useUI } from "../state/AppContext";

const RANGES = [
  { value: "300000", label: "Last 5 minutes" },
  { value: "900000", label: "Last 15 minutes" },
  { value: "3600000", label: "Last hour" },
  { value: "21600000", label: "Last 6 hours" },
  { value: "0", label: "This session" },
];

const columns: Column<HistoryEvent>[] = [
  { key: "time", header: "Time", min: 84, mono: true, render: (e) => <span title={formatDateTime(e.ts)}>{formatClock(e.ts)}</span>, sortValue: (e) => e.ts },
  { key: "kind", header: "Event", min: 230, render: (e) => <EventBadge kind={e.kind} />, sortValue: (e) => e.kind },
  { key: "process", header: "Application", min: 150, grow: 0.8, render: (e) => <span className={e.processName ? "" : "muted"}>{processLabel(e.processName, e.pid)}</span>, sortValue: (e) => e.processName?.toLowerCase() ?? null },
  { key: "protocol", header: "Proto", min: 66, render: (e) => <ProtocolBadge protocol={e.protocol} />, sortValue: (e) => e.protocol },
  { key: "details", header: "Details", min: 280, grow: 1.6, mono: true, render: (e) => <span className="truncate" title={describeEvent(e)}>{describeEvent(e)}</span> },
];

interface Cell { lane: string; start: number; end: number; exclude?: string[] }

function LaneChart({ events, from, to, selected, onPick }: {
  events: HistoryEvent[]; from: number; to: number; selected: Cell | null; onPick: (c: Cell | null) => void;
}) {
  const wrap = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(800);
  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setWidth(Math.max(320, el.clientWidth)));
    ro.observe(el);
    setWidth(Math.max(320, el.clientWidth));
    return () => ro.disconnect();
  }, []);

  const bucketMs = pickBucketMs(to - from);
  const lanes = useMemo(() => buildLanes(events, from, to, bucketMs), [events, from, to, bucketMs]);
  const labelW = 168, laneH = 26, axisH = 22;
  const plotW = Math.max(100, width - labelW - 8);
  const n = lanes[0]?.buckets.length ?? 1;
  const cellW = plotW / n;
  const max = Math.max(1, ...lanes.flatMap((l) => l.buckets.map((b) => b.total)));
  const dominant = (t: Record<EventTone, number>): EventTone => (Object.entries(t).sort((a, b) => b[1] - a[1])[0][0] as EventTone);
  const height = axisH + lanes.length * laneH + 4;
  const ticks = [0, 0.5, 1].map((f) => from + (to - from) * f);

  return (
    <div ref={wrap} className="lanes">
      <svg width={width} height={height} role="img" aria-label="Activity by application over time. Use the event list below for keyboard access.">
        {ticks.map((t, i) => {
          const x = labelW + ((t - from) / (to - from)) * plotW;
          return (
            <g key={i}>
              <line x1={x} x2={x} y1={axisH - 4} y2={height} className="lane-grid" />
              <text x={x} y={12} textAnchor={i === 0 ? "start" : i === 2 ? "end" : "middle"} className="lane-tick">{formatClock(t)}</text>
            </g>
          );
        })}
        {lanes.map((lane, li) => {
          const y = axisH + li * laneH;
          return (
            <g key={lane.name}>
              <text x={0} y={y + laneH / 2 + 4} className="lane-label">
                <title>{lane.name} ({lane.total} events)</title>
                {lane.name.length > 22 ? lane.name.slice(0, 21) + "…" : lane.name}
              </text>
              <rect x={labelW} y={y + 3} width={plotW} height={laneH - 6} rx={4} className="lane-track" />
              {lane.buckets.map((b, bi) => {
                if (b.total === 0) return null;
                const isSel = selected?.lane === lane.name && selected.start === b.start;
                return (
                  <rect
                    key={bi} x={labelW + bi * cellW + 0.5} y={y + 3} width={Math.max(2, cellW - 1)} height={laneH - 6} rx={3}
                    className={`lane-cell tone-${dominant(b.tones)} ${isSel ? "sel" : ""}`}
                    style={{ opacity: 0.4 + 0.6 * Math.min(1, b.total / max) }}
                    onClick={() => onPick(isSel ? null : {
                      lane: lane.name, start: b.start, end: b.start + bucketMs,
                      exclude: lane.name === "Other applications" ? lanes.filter((l) => l.name !== lane.name).map((l) => l.name) : undefined,
                    })}
                  >
                    <title>{`${lane.name}: ${b.total} event${b.total === 1 ? "" : "s"} at ${formatClock(b.start)}`}</title>
                  </rect>
                );
              })}
            </g>
          );
        })}
      </svg>
      <div className="legend">
        {(["open", "close", "listen", "process", "state"] as EventTone[]).map((t) => (
          <span key={t}><i className={`swatch tone-${t}`} />{{ open: "Connection opened", close: "Connection closed", listen: "Port started/stopped", process: "Process", state: "State change" }[t]}</span>
        ))}
        <span className="muted">Each block is {bucketMs >= 60_000 ? `${bucketMs / 60_000} min` : `${bucketMs / 1000} s`}; darker means more events.</span>
      </div>
    </div>
  );
}

export function Activity() {
  const { snapshot } = useSnapshot();
  const { search, setSearch, selection, select } = useUI();
  const [events, setEvents] = useState<HistoryEvent[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [range, setRange] = useState("900000");
  const [protocol, setProtocol] = useState("all");
  const [app, setApp] = useState("all");
  const [cell, setCell] = useState<Cell | null>(null);
  const [sort, setSort] = useState<SortState | null>({ key: "time", dir: "desc" });
  const seq = snapshot?.eventSeq ?? 0;
  const now = snapshot?.timestamp ?? Date.now();

  useEffect(() => {
    let alive = true;
    api.recentEvents(5000).then((e) => { if (alive) { setEvents(e); setError(null); } }).catch((e) => alive && setError(errorMessage(e)));
    return () => { alive = false; };
  }, [seq]);

  const from = range === "0" ? (snapshot?.monitoringSince ?? now - 900_000) : now - Number(range);
  const to = now;

  const apps = useMemo(() => [...new Set((events ?? []).map((e) => e.processName).filter((n): n is string => !!n))].sort((a, b) => a.localeCompare(b)), [events]);
  const inRange = useMemo(
    () => (events ?? []).filter((e) => e.ts >= from && e.ts <= to && (protocol === "all" || e.protocol === protocol) && (app === "all" || e.processName === app) && matchesSearch(e, search)),
    [events, from, to, protocol, app, search],
  );
  const listed = useMemo(
    () => (cell
      ? inRange.filter((e) => {
          const name = e.processName ?? "Unavailable";
          const laneMatch = cell.exclude ? !cell.exclude.includes(name) : name === cell.lane;
          return laneMatch && e.ts >= cell.start && e.ts < cell.end;
        })
      : inRange),
    [inRange, cell],
  );
  const rows = useSortedRows(listed, columns, sort);
  const filtersActive = protocol !== "all" || app !== "all" || search.trim() !== "" || cell !== null;
  const reset = () => { setProtocol("all"); setApp("all"); setSearch(""); setCell(null); };

  if (error) return <Banner tone="error">Could not load activity: {error}</Banner>;
  if (!events) return <Spinner label="Loading recent activity…" />;

  return (
    <div className="page page-fill">
      <Toolbar>
        <Select label="Time range" value={range} onChange={(v) => { setRange(v); setCell(null); }} options={RANGES} />
        <Select label="Protocol" value={protocol} onChange={setProtocol} options={[{ value: "all", label: "All" }, { value: "TCP", label: "TCP" }, { value: "UDP", label: "UDP" }]} />
        <Select label="Application" value={app} onChange={setApp} options={[{ value: "all", label: "All applications" }, ...apps.map((a) => ({ value: a, label: a }))]} />
        <div className="toolbar-spacer" />
        <span className="muted count">{listed.length.toLocaleString()} events</span>
        <Button icon={<FilterX size={15} />} onClick={reset} disabled={!filtersActive}>Reset filters</Button>
      </Toolbar>
      {events.length === 0 ? (
        <EmptyState icon={<ActivityIcon size={28} />} title="No activity detected yet">
          PortWatch lists changes it observes while running: new connections, ports opening or closing and processes starting to use the network.
          Connections that already existed when PortWatch started are not shown as events.
        </EmptyState>
      ) : inRange.length === 0 ? (
        <EmptyState icon={<ActivityIcon size={28} />} title="No events match these filters" action={<Button onClick={reset}>Reset filters</Button>}>
          Try a longer time range or clear the filters.
        </EmptyState>
      ) : (
        <>
          <section className="card lane-card">
            <LaneChart events={inRange} from={from} to={to} selected={cell} onPick={setCell} />
            {cell && <p className="fine">Showing {cell.lane} from {formatClock(cell.start)} to {formatClock(cell.end)}. Click the block again to clear.</p>}
          </section>
          <DataTable
            caption="Activity events" rows={rows} columns={columns} rowKey={(e) => String(e.id)} sort={sort} onSort={(k) => setSort((s) => nextSort(s, k))}
            selectedKey={selection?.kind === "event" ? String(selection.event.id) : null}
            onSelect={(e) => select({ kind: "event", event: e })}
          />
        </>
      )}
      <p className="fine">Activity covers this session only (up to the latest 5,000 events). Use History for what is stored on disk.</p>
    </div>
  );
}

