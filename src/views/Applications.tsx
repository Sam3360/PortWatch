import { useMemo, useState } from "react";
import { AppWindow, Download, FilterX } from "lucide-react";
import { formatDateTime, formatRelative } from "../lib/format";
import { stamp, useCsvExport } from "../lib/hooks";
import type { SortState } from "../lib/sort";
import type { ApplicationRecord } from "../lib/types";
import { Button, Switch, Toolbar } from "../components/Controls";
import { DataTable, nextSort, useSortedRows, type Column } from "../components/DataTable";
import { EmptyState, Spinner } from "../components/Feedback";
import { useSnapshot, useUI } from "../state/AppContext";

function matchesApp(a: ApplicationRecord, query: string): boolean {
  const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
  const hay = [a.name ?? "", String(a.pid), a.path ?? "", ...a.listeningPorts.map(String)].join("\n").toLowerCase();
  return terms.every((t) => hay.includes(t.replace(/^(app|pid|port):/, "")));
}

export function Applications() {
  const { snapshot } = useSnapshot();
  const { search, setSearch, selection, select } = useUI();
  const exportCsv = useCsvExport();
  const [activeOnly, setActiveOnly] = useState(true);
  const [sort, setSort] = useState<SortState | null>({ key: "connections", dir: "desc" });
  const now = snapshot?.timestamp ?? Date.now();

  const columns = useMemo<Column<ApplicationRecord>[]>(() => [
    { key: "name", header: "Application", min: 150, grow: 1, render: (a) => <span className={a.name ? "" : "muted"}>{a.name ?? "Unavailable"}</span>, sortValue: (a) => a.name?.toLowerCase() ?? null },
    { key: "pid", header: "PID", min: 64, align: "right", mono: true, render: (a) => a.pid, sortValue: (a) => a.pid },
    { key: "path", header: "Executable path", min: 220, grow: 1.6, mono: true, render: (a) => <span title={a.path ?? ""} className={a.path ? "truncate" : "muted"}>{a.path ?? "Unavailable"}</span>, sortValue: (a) => a.path?.toLowerCase() ?? null },
    { key: "connections", header: "Connections", min: 96, align: "right", mono: true, render: (a) => a.connectionCount, sortValue: (a) => a.connectionCount },
    { key: "observed", header: "Seen in total", min: 96, align: "right", mono: true, render: (a) => a.observedConnections.toLocaleString(), sortValue: (a) => a.observedConnections },
    { key: "ports", header: "Listening ports", min: 130, grow: 0.7, mono: true, render: (a) => (a.listeningPorts.length ? a.listeningPorts.slice(0, 6).join(", ") + (a.listeningPorts.length > 6 ? ` +${a.listeningPorts.length - 6}` : "") : "—") },
    { key: "last", header: "Last observed", min: 120, render: (a) => (a.active ? <span className="live-text">Active now</span> : <span title={formatDateTime(a.lastSeen)}>{formatRelative(a.lastSeen, now)}</span>), sortValue: (a) => (a.active ? Number.MAX_SAFE_INTEGER : a.lastSeen) },
  ], [now]);

  const filtered = useMemo(
    () => (snapshot?.applications ?? []).filter((a) => (!activeOnly || a.active) && matchesApp(a, search)),
    [snapshot, activeOnly, search],
  );
  const rows = useSortedRows(filtered, columns, sort);
  const filtersActive = !activeOnly || search.trim() !== "";
  const reset = () => { setActiveOnly(true); setSearch(""); };

  if (!snapshot) return <Spinner label="Reading network state…" />;

  const doExport = () => exportCsv(
    `portwatch-applications-${stamp()}.csv`,
    ["Application", "PID", "Executable path", "Current connections", "Connections observed", "Listening ports", "First observed", "Last observed", "Active now"],
    rows.map((a) => [a.name ?? "", a.pid, a.path ?? "", a.connectionCount, a.observedConnections, a.listeningPorts.join(" "), formatDateTime(a.firstSeen), formatDateTime(a.lastSeen), a.active ? "Yes" : "No"]),
  );

  return (
    <div className="page page-fill">
      <Toolbar>
        <div className="inline-switch"><Switch label="Active now only" hint="Hide programs that have since closed their sockets" checked={activeOnly} onChange={setActiveOnly} /></div>
        <div className="toolbar-spacer" />
        <span className="muted count">{rows.length.toLocaleString()} applications</span>
        <Button icon={<FilterX size={15} />} onClick={reset} disabled={!filtersActive}>Reset filters</Button>
        <Button icon={<Download size={15} />} onClick={doExport} disabled={rows.length === 0}>Export CSV</Button>
      </Toolbar>
      {rows.length === 0 ? (
        <EmptyState icon={<AppWindow size={28} />} title={filtersActive ? "No applications match" : "No applications observed yet"}
          action={filtersActive ? <Button onClick={reset}>Reset filters</Button> : undefined}>
          {filtersActive ? "Try a different search, or turn off “Active now only”." : "Applications appear here once they have network sockets."}
        </EmptyState>
      ) : (
        <DataTable
          caption="Applications" rows={rows} columns={columns} rowKey={(a) => String(a.pid)} sort={sort} onSort={(k) => setSort((s) => nextSort(s, k))}
          selectedKey={selection?.kind === "application" ? String(selection.pid) : null}
          onSelect={(a) => select({ kind: "application", pid: a.pid })}
        />
      )}
      <p className="fine">
        Only applications seen while PortWatch was running are listed. Programs that used the network before PortWatch started, or while it was closed, are not included.
        Each row is one process (PID); a program with several processes has several rows.
      </p>
    </div>
  );
}
