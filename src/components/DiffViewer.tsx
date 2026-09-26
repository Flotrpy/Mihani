import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface DiffViewerProps {
  path: string;
  file: string;
  onClose: () => void;
}

export function DiffViewer({ path, file, onClose }: DiffViewerProps) {
  const [diff, setDiff] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDiff(null);
    setError(null);
    invoke<string>("git_diff", { path, file })
      .then(setDiff)
      .catch((err) => setError(String(err)));
  }, [path, file]);

  return (
    <div className="diff-overlay" onClick={onClose}>
      <div className="diff-panel" onClick={(e) => e.stopPropagation()}>
        <div className="diff-panel-header">
          <span className="diff-panel-title">{file}</span>
          <button className="diff-panel-close" onClick={onClose} aria-label="Close diff view">
            ×
          </button>
        </div>
        <pre className="diff-panel-body">
          {error && <span className="diff-error">{error}</span>}
          {!error && diff === null && "Loading…"}
          {!error &&
            diff !== null &&
            (diff.length === 0 ? "No changes" : renderDiffLines(diff))}
        </pre>
      </div>
    </div>
  );
}

function renderDiffLines(diff: string) {
  return diff.split("\n").map((line, i) => {
    let className = "diff-line";
    if (line.startsWith("+")) className += " diff-line-add";
    else if (line.startsWith("-")) className += " diff-line-del";
    else if (line.startsWith("@@")) className += " diff-line-hunk";
    return (
      <div key={i} className={className}>
        {line}
      </div>
    );
  });
}
