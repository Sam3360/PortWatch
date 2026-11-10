import { useId, type ButtonHTMLAttributes, type ReactNode, type Ref, type SelectHTMLAttributes } from "react";
import { Search, X } from "lucide-react";

export function Tip({ text, children }: { text: string; children: ReactNode }) {
  return <span className="tip" data-tip={text}>{children}</span>;
}

type BtnProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "default" | "primary" | "danger" | "ghost";
  icon?: ReactNode;
};

export function Button({ variant = "default", icon, children, className = "", ...rest }: BtnProps) {
  return (
    <button type="button" className={`btn btn-${variant} ${className}`} {...rest}>
      {icon}
      {children}
    </button>
  );
}

export function IconButton({ label, children, ...rest }: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  return (
    <Tip text={label}>
      <button type="button" className="icon-btn" aria-label={label} {...rest}>{children}</button>
    </Tip>
  );
}

export function Switch({
  checked, onChange, label, hint, disabled,
}: { checked: boolean; onChange: (v: boolean) => void; label: string; hint?: string; disabled?: boolean }) {
  const id = useId();
  return (
    <div className="setting-row">
      <div className="setting-text">
        <label htmlFor={id} className="setting-label">{label}</label>
        {hint && <p className="setting-hint">{hint}</p>}
      </div>
      <button
        id={id} type="button" role="switch" aria-checked={checked} disabled={disabled}
        className={`switch ${checked ? "on" : ""}`} onClick={() => onChange(!checked)}
      >
        <span className="switch-knob" />
      </button>
    </div>
  );
}

export function Select({
  label, value, onChange, options, ...rest
}: { label: string; value: string; onChange: (v: string) => void; options: { value: string; label: string }[] }
  & Omit<SelectHTMLAttributes<HTMLSelectElement>, "onChange" | "value">) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      <select className="select" value={value} onChange={(e) => onChange(e.target.value)} {...rest}>
        {options.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
      </select>
    </label>
  );
}

export function TextField({
  label, value, onChange, placeholder, type = "text", width, inputMode, ...rest
}: {
  label: string; value: string; onChange: (v: string) => void; placeholder?: string;
  type?: string; width?: number; inputMode?: "numeric" | "text";
} & { min?: number; max?: number }) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      <input
        className="input" type={type} value={value} placeholder={placeholder} inputMode={inputMode}
        style={width ? { width } : undefined} onChange={(e) => onChange(e.target.value)} {...rest}
      />
    </label>
  );
}

export function SearchBox({
  value, onChange, placeholder, inputRef,
}: { value: string; onChange: (v: string) => void; placeholder: string; inputRef?: Ref<HTMLInputElement> }) {
  return (
    <div className="searchbox">
      <Search size={15} aria-hidden />
      <input
        ref={inputRef} className="searchbox-input" value={value} placeholder={placeholder}
        aria-label="Search" onChange={(e) => onChange(e.target.value)}
      />
      {value && (
        <button type="button" className="searchbox-clear" aria-label="Clear search" onClick={() => onChange("")}>
          <X size={14} />
        </button>
      )}
    </div>
  );
}

export function Segmented<T extends string>({
  label, value, onChange, options,
}: { label: string; value: T; onChange: (v: T) => void; options: { value: T; label: string }[] }) {
  return (
    <div className="field">
      <span className="field-label">{label}</span>
      <div className="seg" role="radiogroup" aria-label={label}>
        {options.map((o) => (
          <button
            key={o.value} type="button" role="radio" aria-checked={value === o.value}
            className={`seg-item ${value === o.value ? "active" : ""}`} onClick={() => onChange(o.value)}
          >
            {o.label}
          </button>
        ))}
      </div>
    </div>
  );
}

export function Toolbar({ children }: { children: ReactNode }) {
  return <div className="toolbar" role="group" aria-label="Filters">{children}</div>;
}
