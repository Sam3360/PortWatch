import { describe, expect, it } from "vitest";
import { bucketEvents, buildLanes, pickBucketMs } from "./timeline";
import type { HistoryEvent } from "./types";

const ev = (ts: number, kind: HistoryEvent["kind"], processName: string | null): HistoryEvent => ({
  id: ts, ts, kind, protocol: "TCP", localAddress: "10.0.0.2", localPort: 1, remoteAddress: null,
  remotePort: null, state: null, prevState: null, pid: 1, processName,
});

describe("timeline", () => {
  it("picks bucket sizes by range", () => {
    expect(pickBucketMs(30_000)).toBe(1_000);
    expect(pickBucketMs(15 * 60_000)).toBe(30_000);
    expect(pickBucketMs(24 * 3_600_000)).toBe(1_800_000);
    expect(pickBucketMs(30 * 24 * 3_600_000)).toBe(3_600_000);
  });
  it("buckets events and classifies tones", () => {
    const b = bucketEvents([ev(500, "connection_opened", "a"), ev(1500, "listening_started", "a"), ev(1600, "connection_closed", "a")], 0, 3000, 1000);
    expect(b.map((x) => x.total)).toEqual([1, 2, 0]);
    expect(b[1].tones.listen).toBe(1);
    expect(b[1].tones.close).toBe(1);
  });
  it("ignores events outside the range", () => {
    const b = bucketEvents([ev(-5, "connection_opened", "a"), ev(9999, "connection_opened", "a")], 0, 2000, 1000);
    expect(b.reduce((s, x) => s + x.total, 0)).toBe(0);
  });
  it("builds lanes for the busiest apps plus an Other lane", () => {
    const events = [ev(1, "connection_opened", "a"), ev(2, "connection_opened", "a"), ev(3, "connection_opened", "b"), ev(4, "connection_opened", "c")];
    const lanes = buildLanes(events, 0, 10, 5, 2);
    expect(lanes.map((l) => l.name)).toEqual(["a", "b", "Other applications"]);
    expect(lanes[0].total).toBe(2);
    expect(buildLanes([], 0, 10, 5)).toEqual([]);
  });
});
