import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface GitBranch {
  name: string;
  is_head: boolean;
}

export function BranchSwitcher({ path, refreshKey }: { path: string; refreshKey: number }) {
  const [branches, setBranches] = useState<GitBranch[]>([]);
  const [creating, setCreating] = useState(false);
  const [newBranch, setNewBranch] = useState("");
  const [error, setError] = useState<string | null>(null);

  const refresh = () => {
    invoke<GitBranch[]>("git_list_branches", { path })
      .then(setBranches)
      .catch(() => setBranches([]));
  };

  useEffect(refresh, [path, refreshKey]);

  const handleSwitch = async (name: string) => {
    setError(null);
    try {
      await invoke("git_checkout_branch", { path, branch: name });
      refresh();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleCreate = async () => {
    if (!newBranch.trim()) return;
    setError(null);
    try {
      await invoke("git_create_branch", { path, branch: newBranch.trim() });
      setNewBranch("");
      setCreating(false);
      refresh();
    } catch (err) {
      setError(String(err));
    }
  };

  const current = branches.find((b) => b.is_head);

  return (
    <div className="branch-switcher">
      <select
        className="branch-select"
        value={current?.name ?? ""}
        onChange={(e) => handleSwitch(e.target.value)}
      >
        {branches.map((b) => (
          <option key={b.name} value={b.name}>
            {b.name}
          </option>
        ))}
      </select>
      {creating ? (
        <div className="branch-create-row">
          <input
            autoFocus
            className="branch-create-input"
            placeholder="new-branch-name"
            value={newBranch}
            onChange={(e) => setNewBranch(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleCreate()}
          />
          <button onClick={handleCreate}>Create</button>
          <button onClick={() => setCreating(false)}>Cancel</button>
        </div>
      ) : (
        <button className="branch-new-btn" onClick={() => setCreating(true)}>
          + New branch
        </button>
      )}
      {error && <div className="git-action-error">{error}</div>}
    </div>
  );
}
