import type { HistoryEvent } from "./types";
import { eventTone, type EventTone } from "./events";

const STEPS = [1_000, 5_000, 10_000, 30_000, 60_000, 300_000, 600_000, 900_000, 1_800_000, 3_600_000];

/** Smallest "nice" bucket size that keeps the chart at or under ~`target` buckets. */
export function pickBucketMs(rangeMs: number, target = 60): number {
  for (const s of STEPS) if (rangeMs / s <= target) return s;
  return STEPS[STEPS.length - 1];
}

export interface Bucket {
  start: number;
  total: number;
  tones: Record<EventTone, number>;
}

const emptyTones = (): Record<EventTone, number> => ({ open: 0, close: 0, listen: 0, process: 0, state: 0 });

export function bucketEvents(events: HistoryEvent[], from: number, to: number, bucketMs: number): Bucket[] {
  const first = Math.floor(from / bucketMs) * bucketMs;
  const n = Math.max(1, Math.ceil((to - first) / bucketMs));
  const buckets: Bucket[] = Array.from({ length: n }, (_, i) => ({ start: first + i * bucketMs, total: 0, tones: emptyTones() }));
  for (const e of events) {
    if (e.ts < from || e.ts > to) continue;
    const i = Math.min(n - 1, Math.floor((e.ts - first) / bucketMs));
    buckets[i].total++;
    buckets[i].tones[eventTone(e.kind)]++;
  }
  return buckets;
}

export interface Lane {
  name: string;
  total: number;
  buckets: Bucket[];
}

/** One lane per busiest application plus a combined "Other" lane. */
export function buildLanes(events: HistoryEvent[], from: number, to: number, bucketMs: number, maxLanes = 8): Lane[] {
  const counts = new Map<string, number>();
  const nameOf = (e: HistoryEvent) => e.processName ?? "Unavailable";
  for (const e of events) counts.set(nameOf(e), (counts.get(nameOf(e)) ?? 0) + 1);
  const ranked = [...counts.entries()].sort((a, b) => b[1] - a[1]);
  const top = new Set(ranked.slice(0, maxLanes).map(([n]) => n));
  const lanes: Lane[] = ranked.slice(0, maxLanes).map(([name, total]) => ({
    name,
    total,
    buckets: bucketEvents(events.filter((e) => nameOf(e) === name), from, to, bucketMs),
  }));
  const rest = events.filter((e) => !top.has(nameOf(e)));
  if (rest.length > 0) lanes.push({ name: "Other applications", total: rest.length, buckets: bucketEvents(rest, from, to, bucketMs) });
  return lanes;
}
