import type { EventKind } from "../lib/types";
import { eventLabel, eventTone } from "../lib/events";

export function ProtocolBadge({ protocol }: { protocol: string }) {
  if (!protocol) return <span className="muted">—</span>;
  return <span className={`badge proto-${protocol.toLowerCase()}`}>{protocol}</span>;
}

export function StateBadge({ state }: { state: string }) {
  const tone =
    state === "Established" ? "good"
    : state === "Listen" || state === "Bound" ? "accent"
    : state === "Time wait" || state === "Closed" || state === "Delete TCB" ? "muted"
    : "warn";
  return <span className={`badge state-${tone}`}><i className="dot" />{state}</span>;
}

export function EventBadge({ kind }: { kind: EventKind }) {
  return <span className={`badge ev-${eventTone(kind)}`}>{eventLabel(kind)}</span>;
}
