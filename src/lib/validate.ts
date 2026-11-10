import type { Settings } from "./types";

export const MIN_INTERVAL_MS = 500;
export const MAX_INTERVAL_MS = 60_000;

/** Mirrors Settings::validate in Rust; the backend re-validates everything. */
export function validateSettings(s: Settings): string[] {
  const errors: string[] = [];
  if (!Number.isInteger(s.refreshIntervalMs) || s.refreshIntervalMs < MIN_INTERVAL_MS || s.refreshIntervalMs > MAX_INTERVAL_MS)
    errors.push(`Refresh interval must be a whole number between ${MIN_INTERVAL_MS} and ${MAX_INTERVAL_MS} ms.`);
  if (!Number.isInteger(s.history.retentionDays) || s.history.retentionDays < 1 || s.history.retentionDays > 3650)
    errors.push("History retention must be between 1 and 3650 days.");
  if (!Number.isInteger(s.history.maxEvents) || s.history.maxEvents < 1000 || s.history.maxEvents > 5_000_000)
    errors.push("Maximum history events must be between 1,000 and 5,000,000.");
  if (s.notifications.monitoredPorts.length > 100) errors.push("At most 100 monitored ports are allowed.");
  return errors;
}

export type PortParse = { ports: number[]; invalid: string[] };

/** Parses "80, 443 8080" into unique valid ports and the rejected tokens. */
export function parsePortList(text: string): PortParse {
  const ports = new Set<number>();
  const invalid: string[] = [];
  for (const token of text.split(/[\s,;]+/).filter(Boolean)) {
    const n = Number(token);
    if (/^\d+$/.test(token) && n >= 1 && n <= 65535) ports.add(n);
    else invalid.push(token);
  }
  return { ports: [...ports].sort((a, b) => a - b), invalid };
}
