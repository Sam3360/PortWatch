import type { ApplicationRecord, Connection } from "./types";

export interface OverviewStats {
  active: number;
  total: number;
  tcp: number;
  udp: number;
  listeningTcp: number;
  uniqueApplications: number;
}

export function computeStats(conns: Connection[], apps: ApplicationRecord[]): OverviewStats {
  let active = 0, tcp = 0, udp = 0, listeningTcp = 0;
  for (const c of conns) {
    if (c.protocol === "UDP") udp++;
    else {
      tcp++;
      if (c.isListening) listeningTcp++;
      else if (c.state === "Established") active++;
    }
  }
  const names = new Set<string>();
  for (const a of apps) if (a.active) names.add((a.name ?? `pid:${a.pid}`).toLowerCase());
  return { active, total: conns.length, tcp, udp, listeningTcp, uniqueApplications: names.size };
}

export function topApplications(apps: ApplicationRecord[], n: number): { name: string; count: number }[] {
  const byName = new Map<string, number>();
  for (const a of apps) {
    if (!a.active) continue;
    const name = a.name ?? `PID ${a.pid}`;
    byName.set(name, (byName.get(name) ?? 0) + a.connectionCount);
  }
  return [...byName.entries()].map(([name, count]) => ({ name, count })).sort((x, y) => y.count - x.count).slice(0, n);
}
