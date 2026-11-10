import { describe, expect, it } from "vitest";
import { matchesSearch, type Searchable } from "./filters";

const item: Searchable = {
  processName: "Chrome.exe", pid: 1234, localAddress: "192.168.1.20", localPort: 51234,
  remoteAddress: "142.250.80.46", remotePort: 443, protocol: "TCP",
};

describe("matchesSearch", () => {
  it("matches an empty query", () => expect(matchesSearch(item, "  ")).toBe(true));
  it("matches name, pid, addresses, ports and protocol", () => {
    for (const q of ["chrome", "1234", "192.168", "142.250", "443", "51234", "tcp"]) expect(matchesSearch(item, q)).toBe(true);
    expect(matchesSearch(item, "firefox")).toBe(false);
  });
  it("requires every term", () => {
    expect(matchesSearch(item, "chrome 443")).toBe(true);
    expect(matchesSearch(item, "chrome 8080")).toBe(false);
  });
  it("supports field prefixes", () => {
    expect(matchesSearch(item, "port:443")).toBe(true);
    expect(matchesSearch(item, "port:44")).toBe(false);
    expect(matchesSearch(item, "pid:1234")).toBe(true);
    expect(matchesSearch(item, "proto:udp")).toBe(false);
    expect(matchesSearch(item, "app:chro")).toBe(true);
    expect(matchesSearch(item, "ip:142.250")).toBe(true);
  });
  it("handles missing names and remote endpoints", () => {
    const bare = { ...item, processName: null, remoteAddress: null, remotePort: null };
    expect(matchesSearch(bare, "chrome")).toBe(false);
    expect(matchesSearch(bare, "51234")).toBe(true);
  });
});
