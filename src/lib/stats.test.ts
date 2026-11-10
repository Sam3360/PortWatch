import { describe, expect, it } from "vitest";
import { computeStats, topApplications } from "./stats";
import type { ApplicationRecord, Connection } from "./types";

const conn = (o: Partial<Connection>): Connection => ({
  id: Math.random().toString(), protocol: "TCP", family: "IPv4", localAddress: "10.0.0.2", localPort: 1,
  remoteAddress: "1.1.1.1", remotePort: 443, state: "Established", pid: 1, processName: "a.exe",
  isListening: false, firstSeen: 0, ...o,
});
const app = (o: Partial<ApplicationRecord>): ApplicationRecord => ({
  pid: 1, name: "a.exe", path: null, startTime: null, firstSeen: 0, lastSeen: 0, active: true,
  connectionCount: 1, listeningPorts: [], observedConnections: 1, ...o,
});

describe("computeStats", () => {
  it("counts each category", () => {
    const stats = computeStats(
      [conn({}), conn({ state: "Time wait" }), conn({ state: "Listen", isListening: true }), conn({ protocol: "UDP", state: "Bound", isListening: true })],
      [app({}), app({ pid: 2, name: "A.EXE" }), app({ pid: 3, name: "b.exe" }), app({ pid: 4, name: "gone.exe", active: false })],
    );
    expect(stats).toMatchObject({ active: 1, total: 4, tcp: 3, udp: 1, listeningTcp: 1, uniqueApplications: 2 });
  });
  it("returns zeros for no data", () => {
    expect(computeStats([], [])).toMatchObject({ active: 0, total: 0, uniqueApplications: 0 });
  });
  it("ranks applications by connections", () => {
    const top = topApplications([app({ name: "x", connectionCount: 2 }), app({ pid: 2, name: "y", connectionCount: 9 }), app({ pid: 3, name: "x", connectionCount: 3 })], 2);
    expect(top).toEqual([{ name: "y", count: 9 }, { name: "x", count: 5 }]);
  });
});
