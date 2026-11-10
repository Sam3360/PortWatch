import { useCallback, useEffect, useState } from "react";
import { api, errorMessage } from "./api";
import { toCsv } from "./csv";
import { useUI } from "../state/AppContext";

export function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = window.setTimeout(() => setV(value), ms);
    return () => window.clearTimeout(id);
  }, [value, ms]);
  return v;
}

/** Builds a CSV from the rows currently shown and lets the user choose where to save it. */
export function useCsvExport() {
  const { toast } = useUI();
  return useCallback(
    async (defaultName: string, headers: string[], rows: (string | number | null | undefined)[][]) => {
      try {
        const path = await api.saveExport(defaultName, toCsv(headers, rows));
        if (path) toast(`Exported ${rows.length.toLocaleString()} rows to ${path}`, "success");
      } catch (e) {
        toast(`Export failed: ${errorMessage(e)}`, "error");
      }
    },
    [toast],
  );
}

export function stamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}`;
}
