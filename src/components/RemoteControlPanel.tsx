import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface RemoteStatus {
  enabled: boolean;
  token: string;
  port: number;
  lan: boolean;
}

export function RemoteControlPanel() {
  const [status, setStatus] = useState<RemoteStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = () => {
    invoke<RemoteStatus>("remote_status").then(setStatus).catch(() => {});
  };

  useEffect(refresh, []);

  const enable = async (lan: boolean) => {
    setBusy(true);
    setError(null);
    try {
      const result = await invoke<RemoteStatus>("remote_enable", { lan });
      setStatus(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const disable = async () => {
    setBusy(true);
    await invoke("remote_disable");
    refresh();
    setBusy(false);
  };

  const regenerateToken = async () => {
    await invoke("remote_regenerate_token");
    refresh();
  };

  if (!status) return <div className="settings-hint">Loading…</div>;

  const url = status.enabled ? `http://<this-device-ip>:${status.port}/?token=${status.token}` : null;

  return (
    <div className="remote-panel">
      <p className="settings-hint">
        View a running agent's terminal output from another device (read-only — this never accepts
        remote keystrokes into your shell). Off by default; when on, it binds to this machine only
        unless you allow LAN access.
      </p>
      {!status.enabled && (
        <div className="agent-add-actions">
          <button disabled={busy} onClick={() => enable(false)}>
            Enable (this device only)
          </button>
          <button disabled={busy} onClick={() => enable(true)}>
            Enable (allow LAN)
          </button>
        </div>
      )}
      {status.enabled && (
        <div className="remote-active">
          <div className="remote-url-label">
            {status.lan ? "Reachable on your local network at:" : "Reachable on this device at:"}
          </div>
          <code className="remote-url">{url}</code>
          {status.lan && (
            <p className="trust-warning">
              Replace &lt;this-device-ip&gt; with this computer's LAN IP address.
            </p>
          )}
          <div className="agent-add-actions">
            <button onClick={regenerateToken}>Regenerate token</button>
            <button onClick={disable} disabled={busy}>
              Disable
            </button>
          </div>
        </div>
      )}
      {error && <div className="git-action-error">{error}</div>}
    </div>
  );
}
