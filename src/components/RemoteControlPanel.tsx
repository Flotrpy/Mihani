import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTestCommand } from "../hooks/useTestCommand";

interface RemoteStatus {
  enabled: boolean;
  token: string;
  port: number;
  lan: boolean;
}

const FIXED_ACTIONS = [
  "Start a built-in agent",
  "Cancel (Ctrl+C)",
  "Restart session",
  "Stop session",
  "Run predefined tests",
  "View terminal output",
  "View git status / diffs",
  "Request commit & push (still requires your local approval)",
];

export function RemoteControlPanel() {
  const [status, setStatus] = useState<RemoteStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { testCommand, setTestCommand } = useTestCommand();
  const [testCommandInput, setTestCommandInput] = useState("");

  const refresh = () => {
    invoke<RemoteStatus>("remote_status").then(setStatus).catch(() => {});
  };

  useEffect(refresh, []);
  useEffect(() => {
    setTestCommandInput(testCommand ?? "");
  }, [testCommand]);

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
        Control and view a running agent from another device using a fixed set of actions only —
        the remote client can never send freeform text or shell commands into a session. Off by
        default; when on, it binds to this machine only unless you allow LAN access.
      </p>
      <ul className="remote-action-list">
        {FIXED_ACTIONS.map((action) => (
          <li key={action}>{action}</li>
        ))}
      </ul>
      <label className="pr-field">
        <span>Test command (used by the remote "Run tests" action)</span>
        <input
          value={testCommandInput}
          onChange={(e) => setTestCommandInput(e.target.value)}
          onBlur={() => testCommandInput.trim() && setTestCommand(testCommandInput.trim())}
          placeholder="e.g. npm test"
        />
      </label>
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
