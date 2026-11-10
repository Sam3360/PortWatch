import type { EventKind, HistoryEvent } from "./types";
import { formatEndpoint, processLabel } from "./format";

export const EVENT_LABELS: Record<EventKind, string> = {
  connection_opened: "Connection first detected",
  connection_closed: "Connection disappeared",
  state_changed: "Connection state changed",
  listening_started: "Port started listening",
  listening_stopped: "Port stopped listening",
  udp_opened: "UDP port became active",
  udp_closed: "UDP port no longer active",
  process_appeared: "Process started using the network",
  process_disappeared: "Process no longer running",
};

export type EventTone = "open" | "close" | "listen" | "process" | "state";

export function eventTone(kind: EventKind): EventTone {
  switch (kind) {
    case "connection_opened": return "open";
    case "connection_closed": return "close";
    case "state_changed": return "state";
    case "listening_started":
    case "listening_stopped":
    case "udp_opened":
    case "udp_closed": return "listen";
    default: return "process";
  }
}

export const eventLabel = (kind: string): string => EVENT_LABELS[kind as EventKind] ?? kind;

/** One-line description, e.g. "chrome.exe (1234) · 10.0.0.2:5000 → 1.1.1.1:443". */
export function describeEvent(e: HistoryEvent): string {
  const who = `${processLabel(e.processName, e.pid)}${e.pid ? ` (${e.pid})` : ""}`;
  if (e.kind === "process_appeared" || e.kind === "process_disappeared") return who;
  const local = formatEndpoint(e.localAddress, e.localPort);
  const remote = e.remoteAddress ? ` → ${formatEndpoint(e.remoteAddress, e.remotePort)}` : "";
  return `${who} · ${e.protocol} ${local}${remote}`;
}
