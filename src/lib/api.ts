import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AlertItem, AppInfo, HistoryEvent, HistoryFilter, HistoryPage, ProcessDetails,
  Settings, Snapshot, StorageInfo,
} from "./types";

/** True when running inside the Tauri shell (false in a plain browser tab). */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Normalises whatever Tauri throws into a readable message. */
export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  try { return JSON.stringify(e); } catch { return "Unknown error"; }
}

export const api = {
  appInfo: () => invoke<AppInfo>("get_app_info"),
  snapshot: () => invoke<Snapshot>("get_snapshot"),
  setPaused: (paused: boolean) => invoke<void>("set_paused", { paused }),
  refreshNow: () => invoke<void>("refresh_now"),
  processDetails: (pid: number) => invoke<ProcessDetails>("get_process_details", { pid }),
  revealProcessFolder: (pid: number) => invoke<void>("reveal_process_folder", { pid }),
  recentEvents: (limit = 5000) => invoke<HistoryEvent[]>("get_recent_events", { limit }),
  queryHistory: (filter: HistoryFilter) => invoke<HistoryPage>("query_history", { filter }),
  historyProcesses: () => invoke<string[]>("list_history_processes"),
  clearHistory: () => invoke<void>("clear_history"),
  saveExport: (defaultName: string, contents: string) =>
    invoke<string | null>("save_export", { defaultName, contents }),
  exportHistory: (filter: HistoryFilter) => invoke<string | null>("export_history_csv", { filter }),
  alerts: (limit = 500) => invoke<AlertItem[]>("list_alerts", { limit }),
  markAlertsRead: () => invoke<void>("mark_alerts_read"),
  clearAlerts: () => invoke<void>("clear_alerts"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  resetSettings: () => invoke<Settings>("reset_settings"),
  storageInfo: () => invoke<StorageInfo>("get_storage_info"),
  openDataFolder: () => invoke<void>("open_data_folder"),
  clearAllData: () => invoke<void>("clear_all_data"),
  openRepository: () => invoke<void>("open_repository"),
  quit: () => invoke<void>("quit_app"),
};

export function onSnapshot(cb: (s: Snapshot) => void): Promise<UnlistenFn> {
  return listen<Snapshot>("snapshot-updated", (e) => cb(e.payload));
}
