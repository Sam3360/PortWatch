import { useMemo, useState } from "react";
import { Download, FilterX, PlugZap } from "lucide-react";
import { matchesSearch } from "../lib/filters";
import { bindingScope, formatDateTime, formatEndpoint, processLabel } from "../lib/format";
import { stamp, useCsvExport } from "../lib/hooks";
import { ipSortKey, type SortState } from "../lib/sort";
import type { Connection } from "../lib/types";
import { ProtocolBadge, StateBadge } from "../components/Badges";
import { Button, Select, Switch, TextField, Toolbar } from "../components/Controls";
import { DataTable, nextSort, useSortedRows, type Column } from "../components/DataTable";
import { Banner, EmptyState, Spinner } from "../components/Feedback";
import { useSnapshot, useUI } from "../state/AppContext";

const columns: Column<Connection>[] = [
  { key: "port", header: "Port", min: 70, mono: true, render: (c) => c.localPort, sortValue: (c) => c.localPort },
  { key: "protocol", header: "Proto", min: 66, render: (c) => <ProtocolBadge protocol={c.protocol} />, sortValue: (c) => c.protocol },
  { key: "process", header: "Application", min: 150, grow: 1, render: (c) => <span className={c.processName ? "" : "muted"}>{processLabel(c.processName, c.pid)}</span>, sortValue: (c) => c.processName?.toLowerCase() ?? null },
  { key: "pid", header: "PID", min: 64, align: "right", mono: true, render: (c) => (c.pid === 0 ? "—" : c.pid), sortValue: (c) => (c.pid === 0 ? null : c.pid) },
  { key: "address", header: "Local address", min: 150, grow: 0.8, mono: true, render: (c) => c.localAddress, sortValue: (c) => ipSortKey(c.localAddress) },
  { key: "scope", header: "Reachable from", min: 160, grow: 0.8, render: (c) => <span className="muted">{bindingScope(c.localAddress)}</span>, sortValue: (c) => bindingScope(c.localAddress) },
  { key: "state", header: "State", min: 116, render: (c) => <StateBadge state={c.state} />, sortValue: (c) => c.state },
  { key: "listening", header: "Listening", min: 80, render: (c) => (c.isListening ? "Yes" : "No"), sortValue: (c) => (c.isListening ? 0 : 1) },
  { key: "first", header: "First observed", min: 150, render: (c) => formatDateTime(c.firstSeen), sortValue: (c) => c.firstSeen },
];

export function Ports() {
  const { snapshot } = useSnapshot();
  const { search, setSearch, selection, select } = useUI();
  const exportCsv = useCsvExport();
  const [protocol, setProtocol] = useState("all");
  const [portText, setPortText] = useState("");
  const [listeningOnly, setListeningOnly] = useState(true);
  const [sort, setSort] = useState<SortState | null>({ key: "port", dir: "asc" });

  const portFilter = /^\d+$/.test(portText.trim()) ? Number(portText.trim()) : null;
  const filtered = useMemo(
    () => (snapshot?.connections ?? []).filter((c) =>
      (!listeningOnly || c.isListening) &&
      (protocol === "all" || c.protocol === protocol) &&
      (portFilter === null || c.localPort === portFilter) &&
      matchesSearch(c, search)),
    [snapshot, listeningOnly, protocol, portFilter, search],
  );
  const rows = useSortedRows(filtered, columns, sort);
  const filtersActive = !listeningOnly || protocol !== "all" || portText !== "" || search.trim() !== "";
  const reset = () => { setListeningOnly(true); setProtocol("all"); setPortText(""); setSearch(""); };

  if (!snapshot) return <Spinner label="Reading network state…" />;

  const doExport = () => exportCsv(
    `portwatch-ports-${stamp()}.csv`,
    ["Port", "Protocol", "Application", "PID", "Local address", "Reachable from", "State", "Listening", "First observed"],
    rows.map((c) => [c.localPort, c.protocol, c.processName ?? "", c.pid || "", c.localAddress, bindingScope(c.localAddress), c.state, c.isListening ? "Yes" : "No", formatDateTime(c.firstSeen)]),
  );

  return (
    <div className="page page-fill">
      <Banner tone="info">
        A listening port is not automatically a security problem. Browsers, games, development tools and Windows itself all open ports.
        Ports bound to “This computer only” cannot be reached from other devices. PortWatch shows what is open, not whether it is safe.
      </Banner>
      <Toolbar>
        <TextField label="Port number" value={portText} onChange={setPortText} placeholder="e.g. 443" width={110} inputMode="numeric" />
        <Select label="Protocol" value={protocol} onChange={setProtocol} options={[{ value: "all", label: "All" }, { value: "TCP", label: "TCP" }, { value: "UDP", label: "UDP" }]} />
        <div className="inline-switch"><Switch label="Listening / bound only" hint="TCP listeners and UDP endpoints" checked={listeningOnly} onChange={setListeningOnly} /></div>
        <div className="toolbar-spacer" />
        <span className="muted count">{rows.length.toLocaleString()} ports</span>
        <Button icon={<FilterX size={15} />} onClick={reset} disabled={!filtersActive}>Reset filters</Button>
        <Button icon={<Download size={15} />} onClick={doExport} disabled={rows.length === 0}>Export CSV</Button>
      </Toolbar>
      {rows.length === 0 ? (
        <EmptyState icon={<PlugZap size={28} />} title={filtersActive ? "No ports match these filters" : "No listening ports reported"}
          action={filtersActive ? <Button onClick={reset}>Reset filters</Button> : undefined}>
          {filtersActive ? "Try a different port, protocol or search." : "Windows did not report any listening ports."}
        </EmptyState>
      ) : (
        <DataTable
          caption="Ports" rows={rows} columns={columns} rowKey={(c) => c.id} sort={sort} onSort={(k) => setSort((s) => nextSort(s, k))}
          selectedKey={selection?.kind === "connection" ? selection.connection.id : null}
          onSelect={(c) => select({ kind: "connection", connection: c, from: "ports" })}
        />
      )}
      <p className="fine">{formatEndpoint("0.0.0.0", 0)} / :: mean “every network interface”. Windows Firewall rules can still block traffic to an open port.</p>
    </div>
  );
}
