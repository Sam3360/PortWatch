import { useMemo, useRef, type KeyboardEvent, type ReactNode } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ArrowDown, ArrowUp } from "lucide-react";
import { sortRows, type SortState, type SortValue } from "../lib/sort";

export interface Column<T> {
  key: string;
  header: string;
  /** Minimum width in px. */
  min: number;
  /** Flex-grow share of leftover space. */
  grow?: number;
  render: (row: T) => ReactNode;
  sortValue?: (row: T) => SortValue;
  align?: "left" | "right";
  mono?: boolean;
}

interface Props<T> {
  rows: T[];
  columns: Column<T>[];
  rowKey: (row: T) => string;
  selectedKey?: string | null;
  onSelect?: (row: T) => void;
  sort: SortState | null;
  onSort: (key: string) => void;
  rowHeight?: number;
  caption: string;
}

/** Applies the current sort to `rows` using the column's sortValue. */
export function useSortedRows<T>(rows: T[], columns: Column<T>[], sort: SortState | null): T[] {
  return useMemo(() => {
    if (!sort) return rows;
    const col = columns.find((c) => c.key === sort.key);
    return sortRows(rows, col?.sortValue, sort.dir);
  }, [rows, columns, sort]);
}

export function nextSort(current: SortState | null, key: string): SortState {
  if (current?.key === key) return { key, dir: current.dir === "asc" ? "desc" : "asc" };
  return { key, dir: "asc" };
}

export function DataTable<T>({ rows, columns, rowKey, selectedKey, onSelect, sort, onSort, rowHeight = 38, caption }: Props<T>) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight,
    overscan: 12,
  });
  const template = columns.map((c) => (c.grow ? `minmax(${c.min}px, ${c.grow}fr)` : `${c.min}px`)).join(" ");
  const minWidth = columns.reduce((s, c) => s + c.min, 0);

  const onRowKey = (e: KeyboardEvent<HTMLDivElement>, row: T) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onSelect?.(row);
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const sib = e.key === "ArrowDown" ? e.currentTarget.nextElementSibling : e.currentTarget.previousElementSibling;
      (sib as HTMLElement | null)?.focus();
    }
  };

  return (
    <div className="dt" role="table" aria-label={caption} aria-rowcount={rows.length + 1}>
      <div className="dt-scroll" ref={scrollRef}>
        <div style={{ minWidth }}>
          <div className="dt-head" role="row" style={{ gridTemplateColumns: template }}>
            {columns.map((c) => {
              const active = sort?.key === c.key;
              return (
                <div
                  key={c.key} role="columnheader" className={`dt-th ${c.align === "right" ? "right" : ""}`}
                  aria-sort={active ? (sort!.dir === "asc" ? "ascending" : "descending") : "none"}
                >
                  {c.sortValue ? (
                    <button type="button" className="dt-sort" onClick={() => onSort(c.key)}>
                      {c.header}
                      {active && (sort!.dir === "asc" ? <ArrowUp size={12} /> : <ArrowDown size={12} />)}
                    </button>
                  ) : c.header}
                </div>
              );
            })}
          </div>
          <div style={{ height: virtualizer.getTotalSize(), position: "relative" }} role="rowgroup">
            {virtualizer.getVirtualItems().map((v) => {
              const row = rows[v.index];
              const key = rowKey(row);
              return (
                <div
                  key={key} role="row" tabIndex={0} aria-rowindex={v.index + 2}
                  aria-selected={selectedKey === key}
                  className={`dt-row ${selectedKey === key ? "selected" : ""}`}
                  style={{ gridTemplateColumns: template, height: rowHeight, transform: `translateY(${v.start}px)` }}
                  onClick={() => onSelect?.(row)}
                  onKeyDown={(e) => onRowKey(e, row)}
                >
                  {columns.map((c) => (
                    <div key={c.key} role="cell" className={`dt-td ${c.mono ? "mono" : ""} ${c.align === "right" ? "right" : ""}`}>
                      {c.render(row)}
                    </div>
                  ))}
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
