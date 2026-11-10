import {
  createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode,
} from "react";
import { api, errorMessage, onSnapshot } from "../lib/api";
import type { Connection, HistoryEvent, Settings, Snapshot } from "../lib/types";

// ---------------------------------------------------------------- snapshot

export interface TrendPoint { t: number; active: number; total: number }
interface SnapshotValue { snapshot: Snapshot | null; loadError: string | null; trend: TrendPoint[] }
const SnapshotCtx = createContext<SnapshotValue>({ snapshot: null, loadError: null, trend: [] });
export const useSnapshot = () => useContext(SnapshotCtx);

const TREND_LIMIT = 120;

function SnapshotProvider({ children }: { children: ReactNode }) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [trend, setTrend] = useState<TrendPoint[]>([]);
  const lastTs = useRef(0);

  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    const accept = (s: Snapshot) => {
      if (!alive) return;
      setSnapshot(s);
      setLoadError(null);
      if (s.hasData && !s.paused && s.timestamp !== lastTs.current) {
        lastTs.current = s.timestamp;
        const active = s.connections.filter((c) => c.protocol === "TCP" && !c.isListening && c.state === "Established").length;
        setTrend((prev) => [...prev.slice(-(TREND_LIMIT - 1)), { t: s.timestamp, active, total: s.connections.length }]);
      }
    };
    api.snapshot().then(accept).catch((e) => alive && setLoadError(errorMessage(e)));
    onSnapshot(accept).then((u) => (alive ? (unlisten = u) : u())).catch((e) => alive && setLoadError(errorMessage(e)));
    return () => { alive = false; unlisten?.(); };
  }, []);

  const value = useMemo(() => ({ snapshot, loadError, trend }), [snapshot, loadError, trend]);
  return <SnapshotCtx.Provider value={value}>{children}</SnapshotCtx.Provider>;
}

// ---------------------------------------------------------------- settings

interface SettingsValue {
  settings: Settings | null;
  save: (s: Settings) => Promise<Settings>;
  reset: () => Promise<Settings>;
  reload: () => Promise<void>;
}
const SettingsCtx = createContext<SettingsValue>({
  settings: null,
  save: async (s) => s,
  reset: async () => { throw new Error("not ready"); },
  reload: async () => {},
});
export const useSettings = () => useContext(SettingsCtx);

function applyTheme(theme: Settings["theme"]) {
  const resolved = theme === "system"
    ? (window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark")
    : theme;
  document.documentElement.setAttribute("data-theme", resolved);
}

function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<Settings | null>(null);

  const reload = useCallback(async () => {
    try { setSettings(await api.getSettings()); } catch { /* surfaced by the snapshot error state */ }
  }, []);
  useEffect(() => { void reload(); }, [reload]);

  useEffect(() => {
    if (!settings) return;
    applyTheme(settings.theme);
    if (settings.theme !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: light)");
    const onChange = () => applyTheme("system");
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [settings?.theme]); // eslint-disable-line react-hooks/exhaustive-deps

  const save = useCallback(async (s: Settings) => {
    const saved = await api.saveSettings(s);
    setSettings(saved);
    return saved;
  }, []);
  const reset = useCallback(async () => {
    const s = await api.resetSettings();
    setSettings(s);
    return s;
  }, []);

  const value = useMemo(() => ({ settings, save, reset, reload }), [settings, save, reset, reload]);
  return <SettingsCtx.Provider value={value}>{children}</SettingsCtx.Provider>;
}

// ---------------------------------------------------------------- UI state

export type Page =
  | "overview" | "connections" | "ports" | "applications" | "activity" | "history" | "alerts" | "settings" | "privacy";

export type Selection =
  | { kind: "connection"; connection: Connection; from: "connections" | "ports" }
  | { kind: "application"; pid: number }
  | { kind: "event"; event: HistoryEvent };

export interface ConfirmOptions { title: string; message: string; confirmLabel?: string; danger?: boolean }
export interface Toast { id: number; message: string; tone: "info" | "success" | "error" }

interface UIValue {
  page: Page;
  setPage: (p: Page) => void;
  search: string;
  setSearch: (s: string) => void;
  selection: Selection | null;
  select: (s: Selection | null) => void;
  toast: (message: string, tone?: Toast["tone"]) => void;
  toasts: Toast[];
  dismissToast: (id: number) => void;
  confirm: (o: ConfirmOptions) => Promise<boolean>;
  pendingConfirm: (ConfirmOptions & { resolve: (ok: boolean) => void }) | null;
}
const UICtx = createContext<UIValue | null>(null);
export function useUI(): UIValue {
  const v = useContext(UICtx);
  if (!v) throw new Error("useUI must be used inside AppProviders");
  return v;
}

function UIProvider({ children }: { children: ReactNode }) {
  const [page, setPageState] = useState<Page>("overview");
  const [search, setSearch] = useState("");
  const [selection, select] = useState<Selection | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [pendingConfirm, setPending] = useState<UIValue["pendingConfirm"]>(null);
  const nextId = useRef(1);

  const setPage = useCallback((p: Page) => { setPageState(p); select(null); }, []);
  const dismissToast = useCallback((id: number) => setToasts((t) => t.filter((x) => x.id !== id)), []);
  const toast = useCallback((message: string, tone: Toast["tone"] = "info") => {
    const id = nextId.current++;
    setToasts((t) => [...t.slice(-3), { id, message, tone }]);
    window.setTimeout(() => dismissToast(id), tone === "error" ? 8000 : 4000);
  }, [dismissToast]);
  const confirm = useCallback(
    (o: ConfirmOptions) => new Promise<boolean>((resolve) => setPending({ ...o, resolve: (ok) => { setPending(null); resolve(ok); } })),
    [],
  );

  const value = useMemo<UIValue>(
    () => ({ page, setPage, search, setSearch, selection, select, toast, toasts, dismissToast, confirm, pendingConfirm }),
    [page, setPage, search, selection, toast, toasts, dismissToast, confirm, pendingConfirm],
  );
  return <UICtx.Provider value={value}>{children}</UICtx.Provider>;
}

export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <SettingsProvider>
      <SnapshotProvider>
        <UIProvider>{children}</UIProvider>
      </SnapshotProvider>
    </SettingsProvider>
  );
}
