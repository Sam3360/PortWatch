import { describe, expect, it } from "vitest";
import { ipSortKey, sortRows } from "./sort";

describe("sortRows", () => {
  const rows = [{ n: "b", v: 2 }, { n: "a", v: 10 }, { n: "c", v: null }, { n: "d", v: 1 }];
  it("sorts numbers ascending and descending with empties last", () => {
    expect(sortRows(rows, (r) => r.v, "asc").map((r) => r.n)).toEqual(["d", "b", "a", "c"]);
    expect(sortRows(rows, (r) => r.v, "desc").map((r) => r.n)).toEqual(["a", "b", "d", "c"]);
  });
  it("sorts strings naturally", () => {
    const r = [{ s: "port10" }, { s: "port9" }];
    expect(sortRows(r, (x) => x.s, "asc").map((x) => x.s)).toEqual(["port9", "port10"]);
  });
  it("is stable and does not mutate the input", () => {
    const input = [{ k: 1, i: 0 }, { k: 1, i: 1 }];
    const out = sortRows(input, (x) => x.k, "desc");
    expect(out.map((x) => x.i)).toEqual([0, 1]);
    expect(input[0].i).toBe(0);
  });
  it("orders IPv4 addresses numerically", () => {
    expect(ipSortKey("10.0.0.9")! < ipSortKey("10.0.0.10")!).toBe(true);
    expect(ipSortKey(null)).toBeNull();
  });
});
