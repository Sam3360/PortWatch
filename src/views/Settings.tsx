import { useEffect, useMemo, useState, type ReactNode } from "react";
import { FolderOpen, RotateCcw, Save, Trash2, Undo2 } from "lucide-react";
import { api, errorMessage } from "../lib/api";
import { formatBytes } from "../lib/format";
import type { Settings as SettingsT, StorageInfo } from "../lib/types";
import { parsePortList, validateSettings } from "../lib/validate";
import { Button, Segmented, Switch } from "../components/Controls";
import { Banner, Spinner } from "../components/Feedback";
import { useSettings, useSnapshot, useUI } from "../state/AppContext";

function Section({ title, description, children }: { title: string; description?: string; children: ReactNode }) {
  return (
    <section className="card">
      <div className="card-head"><div><h2>{title}</h2>{description && <p className="muted">{description}</p>}</div></div>
      {children}
    </section>
  );
}

export function Settings() {
  const { settings, save, reset, reload } = useSettings();
  const { snapshot } = useSnapshot();
  const { confirm, toast, setPage } = useUI();
  const [draft, setDraft] = useState<SettingsT | null>(null);
  const [portsText, setPortsText] = useState("");
  const [intervalText, setIntervalText] = useState("");
  const [retentionText, setRetentionText] = useState("");
  const [maxText, setMaxText] = useState("");
  const [info, setInfo] = useState<StorageInfo | null>(null);
  const [saving, setSaving] = useState(false);

  const load = (s: SettingsT) => {
    setDraft(s);
    setPortsText(s.notifications.monitoredPorts.join(", "));
    setIntervalText(String(s.refreshIntervalMs));
    setRetentionText(String(s.history.retentionDays));
    setMaxText(String(s.history.maxEvents));
  };
  useEffect(() => { if (settings && !draft) load(settings); }, [settings]); // eslint-disable-line react-hooks/exhaustive-deps
  const refreshInfo = () => api.storageInfo().then(setInfo).catch(() => {});
  useEffect(() => { void refreshInfo(); }, []);

  const parsedPorts = useMemo(() => parsePortList(portsText), [portsText]);
  const effective = useMemo<SettingsT | null>(() => draft && ({
    ...draft,
    refreshIntervalMs: Number(intervalText),
    notifications: { ...draft.notifications, monitoredPorts: parsedPorts.ports },
    history: { ...draft.history, retentionDays: Number(retentionText), maxEvents: Number(maxText) },
  }), [draft, intervalText, retentionText, maxText, parsedPorts]);

  const errors = useMemo(() => {
    if (!effective) return [];
    const e = validateSettings(effective);
    if (parsedPorts.invalid.length) e.push(`Not valid ports: ${parsedPorts.invalid.join(", ")} (use whole numbers 1–65535).`);
    return e;
  }, [effective, parsedPorts]);

  const dirty = useMemo(() => !!(effective && settings && JSON.stringify(effective) !== JSON.stringify(settings)), [effective, settings]);

  if (!draft || !effective || !settings) return <Spinner label="Loading settings…" />;
  const set = (patch: Partial<SettingsT>) => setDraft({ ...draft, ...patch });
  const setN = (patch: Partial<SettingsT["notifications"]>) => setDraft({ ...draft, notifications: { ...draft.notifications, ...patch } });
  const setH = (patch: Partial<SettingsT["history"]>) => setDraft({ ...draft, history: { ...draft.history, ...patch } });

  const doSave = async () => {
    if (errors.length) return;
    setSaving(true);
    try { load(await save(effective)); toast("Settings saved.", "success"); void refreshInfo(); }
    catch (e) { toast(errorMessage(e), "error"); }
    finally { setSaving(false); }
  };
  const doReset = async () => {
    if (!(await confirm({ title: "Reset all settings?", message: "Every setting returns to its default, and the “launch at Windows startup” entry is removed. History is not deleted.", confirmLabel: "Reset settings", danger: true }))) return;
    try { load(await reset()); toast("Settings reset to defaults.", "success"); } catch (e) { toast(errorMessage(e), "error"); }
  };
  const run = async (fn: () => Promise<void>, ok: string) => {
    try { await fn(); toast(ok, "success"); void refreshInfo(); } catch (e) { toast(errorMessage(e), "error"); }
  };
  const clearHistory = async () => {
    if (await confirm({ title: "Clear connection history?", message: "Deletes every recorded history event from this computer. This cannot be undone.", confirmLabel: "Clear history", danger: true }))
      await run(() => api.clearHistory(), "History cleared.");
  };
  const clearAlerts = async () => {
    if (await confirm({ title: "Clear notification history?", message: "Deletes the list of past alerts.", confirmLabel: "Clear alerts", danger: true }))
      await run(() => api.clearAlerts(), "Notification history cleared.");
  };
  const clearAll = async () => {
    if (await confirm({ title: "Delete all stored data?", message: "Deletes history, notification history and PortWatch’s memory of which programs it has seen listening (so they may trigger “new listener” alerts again). Settings are kept.", confirmLabel: "Delete everything", danger: true }))
      await run(() => api.clearAllData(), "All stored data deleted.");
  };

  return (
    <div className="page settings-page">
      <div className="save-bar" role="region" aria-label="Save settings">
        <span className={dirty ? "dirty" : "muted"}>{errors.length ? "Fix the highlighted problems to save." : dirty ? "You have unsaved changes." : "All changes saved."}</span>
        <div className="row-gap">
          <Button icon={<Undo2 size={15} />} disabled={!dirty} onClick={() => load(settings)}>Discard</Button>
          <Button variant="primary" icon={<Save size={15} />} disabled={!dirty || errors.length > 0 || saving} onClick={doSave}>Save changes</Button>
        </div>
      </div>
      {errors.length > 0 && <Banner tone="error"><ul className="plain">{errors.map((e) => <li key={e}>{e}</li>)}</ul></Banner>}

      <Section title="Monitoring" description="How often PortWatch reads Windows’ connection tables.">
        <div className="setting-row">
          <div className="setting-text">
            <label className="setting-label" htmlFor="interval">Refresh interval (milliseconds)</label>
            <p className="setting-hint">500–60000. Shorter intervals catch brief connections but use more CPU. 2000 is a good balance.</p>
          </div>
          <input id="interval" className="input" style={{ width: 110 }} inputMode="numeric" value={intervalText} onChange={(e) => setIntervalText(e.target.value)} aria-invalid={errors.some((e) => e.startsWith("Refresh"))} />
        </div>
        <Switch label="Monitoring paused" hint="Pause without closing PortWatch. Changes made while paused are not recorded." checked={snapshot?.paused ?? false}
          onChange={(v) => void api.setPaused(v).catch((e) => toast(errorMessage(e), "error"))} />
      </Section>

      <Section title="General">
        <Segmented label="Theme" value={draft.theme} onChange={(v) => set({ theme: v })} options={[{ value: "dark", label: "Dark" }, { value: "light", label: "Light" }, { value: "system", label: "Match Windows" }]} />
        <Switch label="Launch PortWatch when I sign in to Windows" hint="Off by default. Turning this on adds a per-user startup entry; turning it off removes it. Nothing is changed until you save." checked={draft.launchAtStartup} onChange={(v) => set({ launchAtStartup: v })} />
        <Switch label="Start minimized to the tray" hint="Open without showing the window. Use the tray icon to bring it back." checked={draft.startMinimized} onChange={(v) => set({ startMinimized: v })} />
        <Segmented label="When I close the window" value={draft.closeBehavior} onChange={(v) => set({ closeBehavior: v })} options={[{ value: "tray", label: "Keep running in the tray" }, { value: "exit", label: "Exit PortWatch" }]} />
        <p className="fine">You can always quit fully from the tray menu (“Exit PortWatch”).</p>
      </Section>

      <Section title="Notifications" description="Neutral, local notifications based on what PortWatch actually observed.">
        <Switch label="Enable notifications" checked={draft.notifications.enabled} onChange={(v) => setN({ enabled: v })} />
        <Switch label="Show Windows desktop notifications" hint="Alerts are always added to the Alerts page; this controls the pop-ups. At most 5 pop-ups per minute." checked={draft.notifications.desktopToasts} disabled={!draft.notifications.enabled} onChange={(v) => setN({ desktopToasts: v })} />
        <Switch label="New listening port" hint="A program not seen listening before starts listening on a TCP port." checked={draft.notifications.newListener} disabled={!draft.notifications.enabled} onChange={(v) => setN({ newListener: v })} />
        <Switch label="New outbound connection" hint="First connection from a program to a given remote address. Can be noisy." checked={draft.notifications.newConnection} disabled={!draft.notifications.enabled} onChange={(v) => setN({ newConnection: v })} />
        <Switch label="Monitored port changes" hint="Alert when any of the ports below becomes active or inactive." checked={draft.notifications.monitoredPortChange} disabled={!draft.notifications.enabled} onChange={(v) => setN({ monitoredPortChange: v })} />
        <div className="setting-row">
          <div className="setting-text">
            <label className="setting-label" htmlFor="ports">Monitored ports</label>
            <p className="setting-hint">Separate with commas or spaces, e.g. 22, 3389, 8080.</p>
          </div>
          <input id="ports" className="input" style={{ width: 240 }} value={portsText} onChange={(e) => setPortsText(e.target.value)} placeholder="none" aria-invalid={parsedPorts.invalid.length > 0} />
        </div>
        <Switch label="Listening program exits" hint="A program previously seen listening is no longer running." checked={draft.notifications.processDisappeared} disabled={!draft.notifications.enabled} onChange={(v) => setN({ processDisappeared: v })} />
      </Section>

      <Section title="History" description="Stored only on this computer.">
        <Switch label="Record connection history" hint="When off, nothing new is written to disk (the Activity page still works for this session)." checked={draft.history.enabled} onChange={(v) => setH({ enabled: v })} />
        <Switch label="Record ordinary connections" hint="Open/close/state events for every TCP connection. Turn off to record only listening ports, UDP endpoints and process events." checked={draft.history.recordConnections} disabled={!draft.history.enabled} onChange={(v) => setH({ recordConnections: v })} />
        <Switch label="Store remote addresses" hint="When off, the remote IP and port of a connection are not written to disk." checked={draft.history.recordRemoteAddresses} disabled={!draft.history.enabled} onChange={(v) => setH({ recordRemoteAddresses: v })} />
        <div className="setting-row">
          <div className="setting-text"><label className="setting-label" htmlFor="ret">Keep history for (days)</label><p className="setting-hint">1–3650. Older events are deleted automatically.</p></div>
          <input id="ret" className="input" style={{ width: 110 }} inputMode="numeric" value={retentionText} onChange={(e) => setRetentionText(e.target.value)} />
        </div>
        <div className="setting-row">
          <div className="setting-text"><label className="setting-label" htmlFor="max">Maximum events</label><p className="setting-hint">1,000–5,000,000. The oldest events are removed first.</p></div>
          <input id="max" className="input" style={{ width: 130 }} inputMode="numeric" value={maxText} onChange={(e) => setMaxText(e.target.value)} />
        </div>
      </Section>

      <Section title="Data" description="Everything PortWatch stores lives in one folder on this computer.">
        {info ? (
          <dl className="kvs wide">
            <div className="kv"><dt>Data folder</dt><dd className="mono wrap">{info.dataDir}{info.portable ? " (portable mode)" : ""}</dd></div>
            <div className="kv"><dt>History database</dt><dd className="mono wrap">{info.databasePath}</dd></div>
            <div className="kv"><dt>Settings file</dt><dd className="mono wrap">{info.settingsPath}</dd></div>
            <div className="kv"><dt>Size</dt><dd>{formatBytes(info.databaseSizeBytes)} · {info.eventCount.toLocaleString()} events · {info.alertCount.toLocaleString()} alerts</dd></div>
          </dl>
        ) : <Spinner label="Reading storage details…" />}
        {info?.historyInMemoryOnly && <Banner tone="warn">The history database could not be opened, so history is only kept in memory until PortWatch restarts.</Banner>}
        <div className="row-gap wrap-row">
          <Button icon={<FolderOpen size={15} />} onClick={() => run(() => api.openDataFolder(), "Opened the data folder.")}>Open data folder</Button>
          <Button variant="danger" icon={<Trash2 size={15} />} onClick={clearHistory}>Clear history</Button>
          <Button variant="danger" icon={<Trash2 size={15} />} onClick={clearAlerts}>Clear notification history</Button>
          <Button variant="danger" icon={<Trash2 size={15} />} onClick={clearAll}>Delete all stored data</Button>
        </div>
        <p className="fine">You can also delete the data folder yourself while PortWatch is closed. See the <button type="button" className="link-btn" onClick={() => setPage("privacy")}>Privacy page</button>.</p>
      </Section>

      <Section title="Reset">
        <div className="setting-row">
          <div className="setting-text"><span className="setting-label">Reset settings to defaults</span><p className="setting-hint">Does not delete history.</p></div>
          <Button variant="danger" icon={<RotateCcw size={15} />} onClick={doReset}>Reset settings</Button>
        </div>
        <div className="row-gap"><Button onClick={() => void reload()}>Reload from disk</Button></div>
      </Section>
    </div>
  );
}
