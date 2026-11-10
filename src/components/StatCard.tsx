import type { ReactNode } from "react";

export function StatCard({ label, value, sub, icon }: { label: string; value: ReactNode; sub?: ReactNode; icon: ReactNode }) {
  return (
    <div className="stat">
      <div className="stat-top">
        <span className="stat-label">{label}</span>
        <span className="stat-icon" aria-hidden>{icon}</span>
      </div>
      <div className="stat-value">{value}</div>
      {sub && <div className="stat-sub">{sub}</div>}
    </div>
  );
}

export function Sparkline({ values, height = 56 }: { values: number[]; height?: number }) {
  if (values.length < 2) return <div className="spark-empty">Collecting data…</div>;
  const w = 400;
  const max = Math.max(1, ...values);
  const step = w / (values.length - 1);
  const pts = values.map((v, i) => `${(i * step).toFixed(1)},${(height - 4 - (v / max) * (height - 8)).toFixed(1)}`);
  return (
    <svg className="spark" viewBox={`0 0 ${w} ${height}`} preserveAspectRatio="none" role="img" aria-label={`Trend of ${values.length} readings, peak ${max}`}>
      <polygon points={`0,${height} ${pts.join(" ")} ${w},${height}`} className="spark-fill" />
      <polyline points={pts.join(" ")} className="spark-line" />
    </svg>
  );
}
