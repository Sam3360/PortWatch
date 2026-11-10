import { useEffect, useState } from "react";
import { Github, HardDrive, ShieldCheck } from "lucide-react";
import { api, errorMessage } from "../lib/api";
import type { AppInfo, StorageInfo } from "../lib/types";
import { Button } from "../components/Controls";
import { useUI } from "../state/AppContext";

export function Privacy() {
  const { setPage, toast } = useUI();
  const [info, setInfo] = useState<StorageInfo | null>(null);
  const [app, setApp] = useState<AppInfo | null>(null);
  useEffect(() => {
    api.storageInfo().then(setInfo).catch(() => {});
    api.appInfo().then(setApp).catch(() => {});
  }, []);

  return (
    <div className="page prose">
      <section className="card hero-card">
        <ShieldCheck size={26} aria-hidden />
        <div>
          <h2>Everything stays on this computer</h2>
          <p>PortWatch reads your network state from Windows, shows it to you and optionally keeps a local history. It does not send anything anywhere.</p>
        </div>
      </section>

      <section className="card">
        <h2>What PortWatch does</h2>
        <ul>
          <li><strong>All monitoring is local.</strong> It asks Windows which sockets are open and which program owns each one, using Windows’ own networking APIs.</li>
          <li><strong>No telemetry.</strong> There are no analytics, crash reporters, accounts or update checks. PortWatch makes no network requests of its own, and its web view is restricted so it cannot contact other sites.</li>
          <li><strong>No network data is uploaded.</strong> Nothing about your connections leaves your computer through PortWatch. It does not capture or inspect traffic contents, only which connections exist.</li>
          <li><strong>History is stored locally.</strong> Recorded events (time, protocol, addresses, ports, state, process name and PID) are saved in a SQLite database in the data folder below. Executable paths, command lines and traffic contents are never stored.</li>
          <li><strong>You control your data.</strong> You can turn history off, stop storing remote addresses, choose how long to keep it, and delete it at any time.</li>
        </ul>
      </section>

      <section className="card">
        <h2>Where your data is</h2>
        <div className="kv-block">
          <HardDrive size={16} aria-hidden />
          <span className="mono wrap">{info ? info.dataDir : "Reading…"}{info?.portable ? " (portable mode)" : ""}</span>
        </div>
        <p>It contains <span className="mono">portwatch.db</span> (history, alert history) and <span className="mono">settings.json</span>.</p>
        <h3>How to clear it</h3>
        <ul>
          <li>Open <button type="button" className="link-btn" onClick={() => setPage("history")}>History</button> and choose <strong>Clear history</strong>.</li>
          <li>In <button type="button" className="link-btn" onClick={() => setPage("settings")}>Settings → Data</button>, clear history, clear notification history, or delete all stored data.</li>
          <li>Uninstall PortWatch and delete the data folder above to remove everything.</li>
        </ul>
        <h3>Things you export</h3>
        <p>CSV exports are written only to the location you pick in the Save dialog. After that they are ordinary files that you control.</p>
      </section>

      <section className="card">
        <h2>What PortWatch is not</h2>
        <p>PortWatch is an information tool. It is not a firewall, antivirus or malware detector, and it cannot block or allow connections. A listening port or an unfamiliar program is not proof of a problem, and PortWatch never labels one as malicious.</p>
      </section>

      <section className="card">
        <h2>About</h2>
        <p>PortWatch {app ? `v${app.version}` : ""} is an independent open-source utility by Samarth Chugh, released under the MIT license.</p>
        <Button icon={<Github size={15} />} onClick={() => api.openRepository().catch((e) => toast(errorMessage(e), "error"))}>View source on GitHub</Button>
      </section>
    </div>
  );
}
