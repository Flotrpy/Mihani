import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface GitFileStatus {
  path: string;
  status: string;
}

interface GitRepoStatus {
  branch: string;
  ahead: number;
  behind: number;
  files: GitFileStatus[];
}

const STATUS_LABEL: Record<string, string> = {
  added: "A",
  modified: "M",
  deleted: "D",
  renamed: "R",
  conflicted: "!",
  unknown: "?",
};

export function GitStatusPanel({ path }: { path: string }) {
  const [status, setStatus] = useState<GitRepoStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = () => {
    invoke<GitRepoStatus>("git_status", { path })
      .then((result) => {
        setStatus(result);
        setError(null);
      })
      .catch((err) => setError(String(err)));
  };

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 4000);
    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path]);

  if (error) {
    return <div className="git-panel git-panel-empty">Not a git repository</div>;
  }

  if (!status) {
    return <div className="git-panel git-panel-empty">Loading…</div>;
  }

  return (
    <div className="git-panel">
      <div className="git-panel-header">
        <span className="git-branch">⎇ {status.branch}</span>
        {(status.ahead > 0 || status.behind > 0) && (
          <span className="git-sync">
            {status.ahead > 0 && `↑${status.ahead}`}
            {status.behind > 0 && `↓${status.behind}`}
          </span>
        )}
      </div>
      <ul className="git-file-list">
        {status.files.length === 0 && (
          <li className="git-file-empty">Working tree clean</li>
        )}
        {status.files.map((file) => (
          <li key={file.path} className={`git-file git-file-${file.status}`}>
            <span className="git-file-badge">{STATUS_LABEL[file.status] ?? "?"}</span>
            <span className="git-file-path">{file.path}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
