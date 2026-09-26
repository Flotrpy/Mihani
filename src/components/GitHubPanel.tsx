import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface GitHubUser {
  login: string;
  name: string | null;
  avatar_url: string | null;
}

export function GitHubPanel() {
  const [user, setUser] = useState<GitHubUser | null>(null);
  const [tokenInput, setTokenInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const checkAuth = () => {
    invoke<GitHubUser>("github_whoami")
      .then((u) => {
        setUser(u);
        setError(null);
      })
      .catch(() => setUser(null));
  };

  useEffect(() => {
    checkAuth();
  }, []);

  const handleConnect = async () => {
    if (!tokenInput.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await invoke("github_set_token", { token: tokenInput.trim() });
      setTokenInput("");
      checkAuth();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleDisconnect = async () => {
    await invoke("github_clear_token");
    setUser(null);
  };

  return (
    <div className="github-panel">
      <div className="sidebar-section-title">GitHub</div>
      {user ? (
        <div className="github-user">
          <span className="github-user-login">@{user.login}</span>
          <button className="github-disconnect-btn" onClick={handleDisconnect}>
            Disconnect
          </button>
        </div>
      ) : (
        <div className="github-connect">
          <input
            type="password"
            className="github-token-input"
            placeholder="Personal access token"
            value={tokenInput}
            onChange={(e) => setTokenInput(e.target.value)}
            disabled={busy}
          />
          <button
            className="github-connect-btn"
            onClick={handleConnect}
            disabled={busy || !tokenInput.trim()}
          >
            Connect
          </button>
          {error && <div className="github-error">{error}</div>}
        </div>
      )}
    </div>
  );
}
