export type SortDir = "asc" | "desc";
export interface SortState { key: string; dir: SortDir }
export type SortValue = string | number | null | undefined;

export function compareValues(a: SortValue, b: SortValue): number {
  if (typeof a === "number" && typeof b === "number") return a - b;
  return String(a).localeCompare(String(b), undefined, { numeric: true, sensitivity: "base" });
}

/** Stable sort; empty values always go last regardless of direction. */
export function sortRows<T>(rows: T[], getValue: ((row: T) => SortValue) | undefined, dir: SortDir): T[] {
  if (!getValue) return rows;
  const sign = dir === "asc" ? 1 : -1;
  return rows
    .map((row, index) => ({ row, index, v: getValue(row) }))
    .sort((x, y) => {
      const xe = x.v == null || x.v === "";
      const ye = y.v == null || y.v === "";
      if (xe || ye) return xe === ye ? x.index - y.index : xe ? 1 : -1;
      const c = compareValues(x.v, y.v) * sign;
      return c !== 0 ? c : x.index - y.index;
    })
    .map((x) => x.row);
}

/** Sort key for IP addresses so 10.0.0.9 sorts before 10.0.0.10. */
export function ipSortKey(addr: string | null): string | null {
  if (!addr) return null;
  const a = addr.split("%")[0];
  if (/^\d+\.\d+\.\d+\.\d+$/.test(a)) return "4:" + a.split(".").map((p) => p.padStart(3, "0")).join(".");
  return "6:" + a.toLowerCase();
}
