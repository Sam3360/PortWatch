export function Logo({ size = 28 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" role="img" aria-label="PortWatch logo">
      <rect x="1.5" y="1.5" width="61" height="61" rx="15" className="logo-bg" />
      <circle cx="32" cy="32" r="10.5" className="logo-ring" />
      <circle cx="32" cy="32" r="20" className="logo-ring" />
      <path d="M44.5 13.5 A 20 20 0 0 1 50.4 38" className="logo-arc" />
      <path d="M21.5 38.5 A 10.5 10.5 0 0 1 22.5 25" className="logo-arc" />
      <circle cx="32" cy="32" r="3.6" className="logo-dot" />
    </svg>
  );
}
