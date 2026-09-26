import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CreatePullRequestProps {
  path: string;
  branch: string;
  onClose: () => void;
}

export function CreatePullRequest({ path, branch, onClose }: CreatePullRequestProps) {
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [base, setBase] = useState("main");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<{ number: number; html_url: string } | null>(null);

  const handleSubmit = async () => {
    if (!title.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await invoke("git_push", { path });
      const [owner, repo] = await invoke<[string, string]>("git_remote_info", { path });
      const pr = await invoke<{ number: number; html_url: string }>(
        "github_create_pull_request",
        { owner, repo, title, body, head: branch, base },
      );
      setResult(pr);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="diff-overlay" onClick={onClose}>
      <div className="diff-panel" onClick={(e) => e.stopPropagation()}>
        <div className="diff-panel-header">
          <span className="diff-panel-title">Create Pull Request</span>
          <button className="diff-panel-close" onClick={onClose}>
            ×
          </button>
        </div>
        <div className="pr-form">
          {result ? (
            <div className="pr-success">
              Opened{" "}
              <a href={result.html_url} target="_blank" rel="noreferrer">
                #{result.number}
              </a>
            </div>
          ) : (
            <>
              <label className="pr-field">
                <span>Base branch</span>
                <input value={base} onChange={(e) => setBase(e.target.value)} disabled={busy} />
              </label>
              <label className="pr-field">
                <span>Title</span>
                <input value={title} onChange={(e) => setTitle(e.target.value)} disabled={busy} />
              </label>
              <label className="pr-field">
                <span>Description</span>
                <textarea rows={5} value={body} onChange={(e) => setBody(e.target.value)} disabled={busy} />
              </label>
              {error && <div className="git-action-error">{error}</div>}
              <button
                className="pr-submit-btn"
                onClick={handleSubmit}
                disabled={busy || !title.trim()}
              >
                {busy ? "Pushing & creating…" : "Push & Create PR"}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
