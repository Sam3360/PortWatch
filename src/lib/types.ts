// Mirrors src-tauri/src/models.rs and settings/mod.rs (camelCase JSON).

export interface Connection {
  id: string;
  protocol: "TCP" | "UDP";
  family: "IPv4" | "IPv6";
  localAddress: string;
  localPort: number;
  remoteAddress: string | null;
  remotePort: number | null;
  state: string;
  pid: number;
  processName: string | null;
  isListening: boolean;
  firstSeen: number;
}

export interface ApplicationRecord {
  pid: number;
  name: string | null;
  path: string | null;
  startTime: number | null;
  firstSeen: number;
  lastSeen: number;
  active: boolean;
  connectionCount: number;
  listeningPorts: number[];
  observedConnections: number;
}

export interface Snapshot {
  timestamp: number;
  monitoringSince: number;
  intervalMs: number;
  paused: boolean;
  connections: Connection[];
  applications: ApplicationRecord[];
  warnings: string[];
  error: string | null;
  eventSeq: number;
  unreadAlerts: number;
  platformSupported: boolean;
  hasData: boolean;
}

export type EventKind =
  | "connection_opened"
  | "connection_closed"
  | "state_changed"
  | "listening_started"
  | "listening_stopped"
  | "udp_opened"
  | "udp_closed"
  | "process_appeared"
  | "process_disappeared";

export interface HistoryEvent {
  id: number;
  ts: number;
  kind: EventKind;
  protocol: string;
  localAddress: string;
  localPort: number;
  remoteAddress: string | null;
  remotePort: number | null;
  state: string | null;
  prevState: string | null;
  pid: number;
  processName: string | null;
}

export interface HistoryFilter {
  search?: string;
  from?: number;
  to?: number;
  protocol?: string;
  process?: string;
  kind?: string;
  limit?: number;
  offset?: number;
}

export interface HistoryPage {
  events: HistoryEvent[];
  total: number;
}

export interface AlertItem {
  id: number;
  ts: number;
  category: string;
  title: string;
  message: string;
  processName: string | null;
  pid: number;
  port: number | null;
  protocol: string | null;
  read: boolean;
}

export interface ProcessDetails {
  pid: number;
  name: string | null;
  path: string | null;
  startTime: number | null;
  running: boolean;
  accessible: boolean;
  note: string | null;
}

export interface StorageInfo {
  dataDir: string;
  databasePath: string;
  settingsPath: string;
  databaseSizeBytes: number;
  eventCount: number;
  alertCount: number;
  portable: boolean;
  historyInMemoryOnly: boolean;
}

export interface AppInfo {
  version: string;
  repository: string;
  platformSupported: boolean;
  portable: boolean;
}

export interface NotificationSettings {
  enabled: boolean;
  desktopToasts: boolean;
  newListener: boolean;
  newConnection: boolean;
  monitoredPortChange: boolean;
  processDisappeared: boolean;
  monitoredPorts: number[];
}

export interface HistorySettings {
  enabled: boolean;
  retentionDays: number;
  maxEvents: number;
  recordConnections: boolean;
  recordRemoteAddresses: boolean;
}

export type ThemeSetting = "dark" | "light" | "system";

export interface Settings {
  refreshIntervalMs: number;
  startMinimized: boolean;
  launchAtStartup: boolean;
  closeBehavior: "tray" | "exit";
  theme: ThemeSetting;
  notifications: NotificationSettings;
  history: HistorySettings;
}
