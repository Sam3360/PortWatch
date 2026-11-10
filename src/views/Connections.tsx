import { useMemo, useState } from "react";
import { Download, FilterX, Network } from "lucide-react";
import { matchesSearch } from "../lib/filters";
import { formatClock, formatDateTime, formatDuration, formatEndpoint, processLabel } from "../lib/format";
import { stamp, useCsvExport } from "../lib/hooks";
import { ipSortKey, type SortState } from "../lib/sort";
import type { Connection } from "../lib/types";
import { ProtocolBadge, StateBadge } from "../components/Badges";
import { Button, Select, Toolbar } from "../components/Controls";
import { DataTable, nextSort, useSortedRows, type Column } from "../components/DataTable";
import { Banner, EmptyState, Spinner } from "../components/Feedback";
import { useSnapshot, useUI } from "../state/AppContext";

export function Connections() {
  const { snapshot } = useSnapshot();
  const { search, setSearch, selection, select } = useUI();
  const exportCsv = useCsvExport();
  const [protocol, setProtocol] = useState("all");
  const [state, setState] = useState("all");
  const [family, setFamily] = useState("all");
  const [sort, setSort] = useState<SortState | null>({ key: "process", dir: "asc" });

  const now = snapshot?.timestamp ?? Date.now();
  const columns = useMemo<Column<Connection>[]>(() => [
    { key: "process", header: "Application", min: 150, grow: 1.1, render: (c) => <span title={processLabel(c.processName, c.pid)} className={c.processName ? "" : "muted"}>{processLabel(c.processName, c.pid)}</span>, sortValue: (c) => c.processName?.toLowerCase() ?? null },
    { key: "pid", header: "PID", min: 64, align: "right", render: (c) => (c.pid === 0 ? "—" : c.pid), sortValue: (c) => (c.pid === 0 ? null : c.pid), mono: true },
    { key: "protocol", header: "Proto", min: 66, render: (c) => <ProtocolBadge protocol={c.protocol} />, sortValue: (c) => c.protocol },
    { key: "local", header: "Local address", min: 170, grow: 1, mono: true, render: (c) => formatEndpoint(c.localAddress, c.localPort), sortValue: (c) => (ipSortKey(c.localAddress) ?? "") + String(c.localPort).padStart(5, "0") },
    { key: "remote", header: "Remote address", min: 170, grow: 1, mono: true, render: (c) => formatEndpoint(c.remoteAddress, c.remotePort), sortValue: (c) => (c.remoteAddress ? (ipSortKey(c.remoteAddress) ?? "") + String(c.remotePort ?? 0).padStart(5, "0") : null) },
    { key: "state", header: "State", min: 116, render: (c) => <StateBadge state={c.state} />, sortValue: (c) => c.state },
    { key: "duration", header: "Observed for", min: 92, align: "right", render: (c) => formatDuration(now - c.firstSeen), sortValue: (c) => now - c.firstSeen },
    { key: "first", header: "First observed", min: 100, render: (c) => <span title={formatDateTime(c.firstSeen)}>{formatClock(c.firstSeen)}</span>, sortValue: (c) => c.firstSeen },
    { key: "last", header: "Last observed", min: 100, render: () => <span title={formatDateTime(now)}>{formatClock(now)}</span> },
  ], [now]);

  const states = useMemo(() => [...new Set(snapshot?.connections.map((c) => c.state) ?? [])].sort(), [snapshot]);
  const filtered = useMemo(
    () => (snapshot?.connections ?? []).filter((c) =>
      (protocol === "all" || c.protocol === protocol) &&
      (state === "all" || c.state === state) &&
      (family === "all" || c.family === family) &&
      matchesSearch(c, search)),
    [snapshot, protocol, state, family, search],
  );
  const rows = useSortedRows(filtered, columns, sort);
  const filtersActive = protocol !== "all" || state !== "all" || family !== "all" || search.trim() !== "";
  const reset = () => { setProtocol("all"); setState("all"); setFamily("all"); setSearch(""); };

  if (!snapshot) return <Spinner label="Reading network state…" />;

  const doExport = () => exportCsv(
    `portwatch-connections-${stamp()}.csv`,
    ["Application", "PID", "Protocol", "IP version", "Local address", "Local port", "Remote address", "Remote port", "State", "First observed", "Last observed"],
    rows.map((c) => [c.processName ?? "", c.pid || "", c.protocol, c.family, c.localAddress, c.localPort, c.remoteAddress ?? "", c.remotePort ?? "", c.state, formatDateTime(c.firstSeen), formatDateTime(now)]),
  );

  return (
    <div className="page page-fill">
      {snapshot.error && <Banner tone="error">{snapshot.error}</Banner>}
      <Toolbar>
        <Select label="Protocol" value={protocol} onChange={setProtocol} options={[{ value: "all", label: "All" }, { value: "TCP", label: "TCP" }, { value: "UDP", label: "UDP" }]} />
        <Select label="State" value={state} onChange={setState} options={[{ value: "all", label: "All states" }, ...states.map((s) => ({ value: s, label: s }))]} />
        <Select label="IP version" value={family} onChange={setFamily} options={[{ value: "all", label: "IPv4 and IPv6" }, { value: "IPv4", label: "IPv4" }, { value: "IPv6", label: "IPv6" }]} />
        <div className="toolbar-spacer" />
        <span className="muted count">{rows.length.toLocaleString()} of {snapshot.connections.length.toLocaleString()}</span>
        <Button icon={<FilterX size={15} />} onClick={reset} disabled={!filtersActive}>Reset filters</Button>
        <Button icon={<Download size={15} />} onClick={doExport} disabled={rows.length === 0}>Export CSV</Button>
      </Toolbar>
      {rows.length === 0 ? (
        <EmptyState icon={<Network size={28} />} title={filtersActive ? "No connections match these filters" : "No connections reported"}
          action={filtersActive ? <Button onClick={reset}>Reset filters</Button> : undefined}>
          {filtersActive ? "Try a different search or clear the filters." : "Windows did not report any sockets in the latest reading."}
        </EmptyState>
      ) : (
        <DataTable
          caption="Connections" rows={rows} columns={columns} rowKey={(c) => c.id} sort={sort} onSort={(k) => setSort((s) => nextSort(s, k))}
          selectedKey={selection?.kind === "connection" ? selection.connection.id : null}
          onSelect={(c) => select({ kind: "connection", connection: c, from: "connections" })}
        />
      )}
    </div>
  );
}
