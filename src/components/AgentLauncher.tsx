import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCustomAgents } from "../hooks/useCustomAgents";

export interface AgentDefinition {
  id: string;
  name: string;
  command: string;
  description: string;
}

interface AgentLauncherProps {
  onLaunch: (agent: AgentDefinition) => void;
  onManage: () => void;
}

export function AgentLauncher({ onLaunch, onManage }: AgentLauncherProps) {
  const [builtins, setBuiltins] = useState<AgentDefinition[]>([]);
  const { customAgents, trustedIds, trustAgent } = useCustomAgents();
  const [pendingTrust, setPendingTrust] = useState<AgentDefinition | null>(null);

  useEffect(() => {
    invoke<AgentDefinition[]>("list_agents").then(setBuiltins).catch(() => {});
  }, []);

  const isCustom = (agent: AgentDefinition) => agent.id.startsWith("custom-");

  const handleLaunchClick = (agent: AgentDefinition) => {
    if (isCustom(agent) && !trustedIds.has(agent.id)) {
      setPendingTrust(agent);
      return;
    }
    onLaunch(agent);
  };

  const confirmTrust = async () => {
    if (!pendingTrust) return;
    await trustAgent(pendingTrust.id);
    onLaunch(pendingTrust);
    setPendingTrust(null);
  };

  const allAgents = [...builtins, ...customAgents];

  return (
    <div className="agent-launcher">
      <div className="sidebar-section-title-row">
        <span className="sidebar-section-title">Agents</span>
        <button className="sidebar-manage-btn" onClick={onManage}>
          Manage
        </button>
      </div>
      <ul className="agent-list">
        {allAgents.map((agent) => (
          <li key={agent.id} className="agent-item" title={agent.description}>
            <span className="agent-name">
              {agent.name}
              {isCustom(agent) && <span className="agent-custom-badge">custom</span>}
            </span>
            <button className="agent-launch-btn" onClick={() => handleLaunchClick(agent)}>
              Launch
            </button>
          </li>
        ))}
      </ul>

      {pendingTrust && (
        <div className="diff-overlay" onClick={() => setPendingTrust(null)}>
          <div className="trust-dialog" onClick={(e) => e.stopPropagation()}>
            <p>
              <strong>{pendingTrust.name}</strong> will run this command in your workspace
              terminal:
            </p>
            <code className="trust-command">{pendingTrust.command}</code>
            <p className="trust-warning">
              Only trust commands from sources you control. This runs with your full user
              permissions.
            </p>
            <div className="agent-add-actions">
              <button onClick={confirmTrust}>Trust &amp; Run</button>
              <button onClick={() => setPendingTrust(null)}>Cancel</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
