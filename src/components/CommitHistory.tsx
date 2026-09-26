import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface GitLogEntry {
  oid: string;
  short_oid: string;
  summary: string;
  author: string;
  timestamp: number;
}

function formatRelativeTime(unixSeconds: number): string {
  const diffMs = Date.now() - unixSeconds * 1000;
  const diffMin = Math.round(diffMs / 60000);
  if (diffMin < 1) return "just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.round(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.round(diffHr / 24);
  return `${diffDay}d ago`;
}

export function CommitHistory({ path, refreshKey }: { path: string; refreshKey: number }) {
  const [entries, setEntries] = useState<GitLogEntry[]>([]);

  useEffect(() => {
    invoke<GitLogEntry[]>("git_log", { path, limit: 20 })
      .then(setEntries)
      .catch(() => setEntries([]));
  }, [path, refreshKey]);

  if (entries.length === 0) {
    return <div className="commit-history-empty">No commits yet</div>;
  }

  return (
    <ul className="commit-history-list">
      {entries.map((entry) => (
        <li key={entry.oid} className="commit-history-item" title={entry.summary}>
          <code className="commit-history-oid">{entry.short_oid}</code>
          <span className="commit-history-summary">{entry.summary}</span>
          <span className="commit-history-meta">
            {entry.author} · {formatRelativeTime(entry.timestamp)}
          </span>
        </li>
      ))}
    </ul>
  );
}
