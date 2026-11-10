import { describe, expect, it } from "vitest";
import { parsePortList, validateSettings } from "./validate";
import type { Settings } from "./types";

const base: Settings = {
  refreshIntervalMs: 2000, startMinimized: false, launchAtStartup: false, closeBehavior: "tray", theme: "dark",
  notifications: { enabled: true, desktopToasts: true, newListener: true, newConnection: false, monitoredPortChange: true, processDisappeared: false, monitoredPorts: [] },
  history: { enabled: true, retentionDays: 30, maxEvents: 200000, recordConnections: true, recordRemoteAddresses: true },
};

describe("validateSettings", () => {
  it("accepts defaults", () => expect(validateSettings(base)).toEqual([]));
  it("rejects out-of-range and fractional values", () => {
    expect(validateSettings({ ...base, refreshIntervalMs: 100 })).toHaveLength(1);
    expect(validateSettings({ ...base, refreshIntervalMs: 1000.5 })).toHaveLength(1);
    expect(validateSettings({ ...base, history: { ...base.history, retentionDays: 0, maxEvents: 5 } })).toHaveLength(2);
  });
});

describe("parsePortList", () => {
  it("parses, de-duplicates and sorts", () => {
    expect(parsePortList("443, 80 443;8080")).toEqual({ ports: [80, 443, 8080], invalid: [] });
  });
  it("reports invalid tokens", () => {
    expect(parsePortList("0 65536 abc 22 -1")).toEqual({ ports: [22], invalid: ["0", "65536", "abc", "-1"] });
  });
  it("handles empty input", () => expect(parsePortList("  ")).toEqual({ ports: [], invalid: [] }));
});
