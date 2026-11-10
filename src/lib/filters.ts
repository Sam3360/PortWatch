export interface Searchable {
  processName: string | null;
  pid: number;
  localAddress: string;
  localPort: number;
  remoteAddress: string | null;
  remotePort: number | null;
  protocol: string;
}

/**
 * Space-separated terms, all of which must match (AND).
 * Plain terms match any field as a substring. Prefixes narrow a term:
 * `port:443`, `pid:1234`, `proto:udp`, `ip:10.0.0.`, `app:chrome`.
 */
export function matchesSearch(item: Searchable, query: string): boolean {
  const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) return true;
  const name = (item.processName ?? "").toLowerCase();
  const local = item.localAddress.toLowerCase();
  const remote = (item.remoteAddress ?? "").toLowerCase();
  const ports = [String(item.localPort), item.remotePort != null ? String(item.remotePort) : ""];
  const proto = item.protocol.toLowerCase();
  const pid = String(item.pid);

  return terms.every((t) => {
    const idx = t.indexOf(":");
    const prefix = idx > 0 ? t.slice(0, idx) : "";
    const value = idx > 0 ? t.slice(idx + 1) : t;
    switch (prefix) {
      case "port": return value === "" || ports.some((p) => p === value);
      case "pid": return value === "" || pid === value;
      case "proto": return value === "" || proto === value;
      case "ip": return value === "" || local.includes(value) || remote.includes(value);
      case "app": return value === "" || name.includes(value);
      default:
        return (
          name.includes(t) || pid.includes(t) || local.includes(t) || remote.includes(t) ||
          ports.some((p) => p.includes(t)) || proto.includes(t)
        );
    }
  });
}
