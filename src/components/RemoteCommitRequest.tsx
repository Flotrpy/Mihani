import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface RemoteCommitRequestProps {
  path: string;
  onClose: () => void;
}

export function RemoteCommitRequest({ path, onClose }: RemoteCommitRequestProps) {
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);

  const handleApprove = async () => {
    if (!message.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await invoke("git_commit", { path, message, files: null });
      await invoke("git_push", { path });
      setDone(true);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="diff-overlay" onClick={onClose}>
      <div className="trust-dialog" onClick={(e) => e.stopPropagation()}>
        <p>
          A remote client requested a commit &amp; push for <strong>{path}</strong>.
        </p>
        <p className="trust-warning">
          Nothing happens until you approve this here — the remote client cannot commit or push
          on its own.
        </p>
        {done ? (
          <p>Committed and pushed.</p>
        ) : (
          <>
            <textarea
              className="git-commit-input"
              placeholder="Commit message"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              disabled={busy}
              rows={2}
              autoFocus
            />
            {error && <div className="git-action-error">{error}</div>}
            <div className="agent-add-actions">
              <button onClick={handleApprove} disabled={busy || !message.trim()}>
                Commit &amp; Push
              </button>
              <button onClick={onClose} disabled={busy}>
                Dismiss
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
