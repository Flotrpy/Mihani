import { useState } from "react";
import { GitHubPanel } from "./GitHubPanel";
import { AgentManager } from "./AgentManager";
import { RemoteControlPanel } from "./RemoteControlPanel";

type Tab = "github" | "agents" | "remote";

export function SettingsPanel({ onClose, initialTab }: { onClose: () => void; initialTab?: Tab }) {
  const [tab, setTab] = useState<Tab>(initialTab ?? "github");

  return (
    <div className="diff-overlay" onClick={onClose}>
      <div className="settings-panel" onClick={(e) => e.stopPropagation()}>
        <div className="diff-panel-header">
          <span className="diff-panel-title">Settings</span>
          <button className="diff-panel-close" onClick={onClose} aria-label="Close settings">
            ×
          </button>
        </div>
        <div className="settings-tabs">
          <button
            className={`settings-tab ${tab === "github" ? "settings-tab-active" : ""}`}
            onClick={() => setTab("github")}
          >
            GitHub
          </button>
          <button
            className={`settings-tab ${tab === "agents" ? "settings-tab-active" : ""}`}
            onClick={() => setTab("agents")}
          >
            Agents
          </button>
          <button
            className={`settings-tab ${tab === "remote" ? "settings-tab-active" : ""}`}
            onClick={() => setTab("remote")}
          >
            Remote
          </button>
        </div>
        <div className="settings-body">
          {tab === "github" && <GitHubPanel />}
          {tab === "agents" && <AgentManager />}
          {tab === "remote" && <RemoteControlPanel />}
        </div>
      </div>
    </div>
  );
}
