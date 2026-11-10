import { describe, expect, it } from "vitest";
import { bindingScope, formatBytes, formatDuration, formatEndpoint, formatRelative, processLabel } from "./format";

describe("format", () => {
  it("formats IPv4 and IPv6 endpoints", () => {
    expect(formatEndpoint("10.0.0.1", 443)).toBe("10.0.0.1:443");
    expect(formatEndpoint("::1", 8080)).toBe("[::1]:8080");
    expect(formatEndpoint("fe80::1%12", 80)).toBe("[fe80::1%12]:80");
    expect(formatEndpoint("0.0.0.0", 0)).toBe("0.0.0.0");
    expect(formatEndpoint(null, null)).toBe("—");
  });
  it("formats durations", () => {
    expect(formatDuration(0)).toBe("0s");
    expect(formatDuration(45_000)).toBe("45s");
    expect(formatDuration(192_000)).toBe("3m 12s");
    expect(formatDuration(7_500_000)).toBe("2h 5m");
    expect(formatDuration(100_800_000)).toBe("1d 4h");
    expect(formatDuration(-1)).toBe("—");
  });
  it("describes binding scope", () => {
    expect(bindingScope("127.0.0.1")).toBe("This computer only");
    expect(bindingScope("::1")).toBe("This computer only");
    expect(bindingScope("0.0.0.0")).toBe("All network interfaces");
    expect(bindingScope("::")).toBe("All network interfaces");
    expect(bindingScope("192.168.1.5")).toBe("One specific network interface");
  });
  it("labels processes honestly", () => {
    expect(processLabel("a.exe", 4)).toBe("a.exe");
    expect(processLabel(null, 0)).toBe("No owning process");
    expect(processLabel(null, 99)).toBe("Unavailable");
  });
  it("formats relative times and sizes", () => {
    expect(formatRelative(1000, 2000)).toBe("just now");
    expect(formatRelative(0, 30_000)).toBe("30s ago");
    expect(formatRelative(0, 180_000)).toBe("3m ago");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
  });
});
