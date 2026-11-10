export function formatEndpoint(address: string | null | undefined, port: number | null | undefined): string {
  if (!address) return "—";
  if (!port) return address;
  return address.includes(":") ? `[${address}]:${port}` : `${address}:${port}`;
}

export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "—";
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ${s % 60}s`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ${m % 60}m`;
  const d = Math.floor(h / 24);
  return `${d}d ${h % 24}h`;
}

const pad = (n: number) => String(n).padStart(2, "0");

export function formatClock(ts: number): string {
  const d = new Date(ts);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

export function formatDate(ts: number): string {
  const d = new Date(ts);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function formatDateTime(ts: number | null | undefined): string {
  if (ts == null) return "Unavailable";
  return `${formatDate(ts)} ${formatClock(ts)}`;
}

export function formatRelative(ts: number, now: number): string {
  const diff = Math.max(0, now - ts);
  if (diff < 5_000) return "just now";
  if (diff < 60_000) return `${Math.floor(diff / 1000)}s ago`;
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
  return formatDateTime(ts);
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

const bare = (addr: string) => addr.split("%")[0];

export function isLoopback(addr: string): boolean {
  const a = bare(addr);
  return a === "::1" || a.startsWith("127.");
}

export function isUnspecified(addr: string): boolean {
  const a = bare(addr);
  return a === "0.0.0.0" || a === "::";
}

/** Who can reach a socket bound to this local address (binding scope, not firewall state). */
export function bindingScope(addr: string): string {
  if (isLoopback(addr)) return "This computer only";
  if (isUnspecified(addr)) return "All network interfaces";
  return "One specific network interface";
}

export function processLabel(name: string | null, pid: number): string {
  if (name) return name;
  return pid === 0 ? "No owning process" : "Unavailable";
}

export function plural(n: number, one: string, many = `${one}s`): string {
  return `${n.toLocaleString()} ${n === 1 ? one : many}`;
}
