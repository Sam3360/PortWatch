import { useEffect, useRef, type ReactNode } from "react";
import { AlertTriangle, CheckCircle2, Info, Loader2, X } from "lucide-react";
import { useUI } from "../state/AppContext";

export function EmptyState({ icon, title, children, action }: { icon?: ReactNode; title: string; children?: ReactNode; action?: ReactNode }) {
  return (
    <div className="empty">
      {icon && <div className="empty-icon" aria-hidden>{icon}</div>}
      <h3>{title}</h3>
      {children && <p>{children}</p>}
      {action}
    </div>
  );
}

export function Spinner({ label = "Loading…" }: { label?: string }) {
  return (
    <div className="loading" role="status">
      <Loader2 size={18} className="spin" aria-hidden /> <span>{label}</span>
    </div>
  );
}

export function Banner({ tone = "info", children }: { tone?: "info" | "warn" | "error"; children: ReactNode }) {
  const Icon = tone === "info" ? Info : AlertTriangle;
  return (
    <div className={`banner banner-${tone}`} role={tone === "error" ? "alert" : "status"}>
      <Icon size={16} aria-hidden />
      <div>{children}</div>
    </div>
  );
}

export function ToastHost() {
  const { toasts, dismissToast } = useUI();
  return (
    <div className="toasts" aria-live="polite">
      {toasts.map((t) => (
        <div key={t.id} className={`toast toast-${t.tone}`}>
          {t.tone === "success" ? <CheckCircle2 size={16} /> : t.tone === "error" ? <AlertTriangle size={16} /> : <Info size={16} />}
          <span>{t.message}</span>
          <button type="button" aria-label="Dismiss" onClick={() => dismissToast(t.id)}><X size={14} /></button>
        </div>
      ))}
    </div>
  );
}

/** Native <dialog>: focus is trapped and Escape cancels. */
export function ConfirmDialog() {
  const { pendingConfirm } = useUI();
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const d = ref.current;
    if (!d) return;
    if (pendingConfirm && !d.open) d.showModal();
    if (!pendingConfirm && d.open) d.close();
  }, [pendingConfirm]);

  if (!pendingConfirm) return <dialog ref={ref} className="dialog" />;
  const { title, message, confirmLabel, danger, resolve } = pendingConfirm;
  return (
    <dialog ref={ref} className="dialog" aria-labelledby="dlg-title" onCancel={(e) => { e.preventDefault(); resolve(false); }}>
      <h2 id="dlg-title">{title}</h2>
      <p>{message}</p>
      <div className="dialog-actions">
        <button type="button" className="btn btn-default" autoFocus onClick={() => resolve(false)}>Cancel</button>
        <button type="button" className={`btn ${danger ? "btn-danger" : "btn-primary"}`} onClick={() => resolve(true)}>
          {confirmLabel ?? "Confirm"}
        </button>
      </div>
    </dialog>
  );
}
