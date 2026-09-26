import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface AgentDefinition {
  id: string;
  name: string;
  command: string;
  description: string;
}

interface AgentLauncherProps {
  onLaunch: (agent: AgentDefinition) => void;
}

export function AgentLauncher({ onLaunch }: AgentLauncherProps) {
  const [agents, setAgents] = useState<AgentDefinition[]>([]);

  useEffect(() => {
    invoke<AgentDefinition[]>("list_agents").then(setAgents).catch(() => {});
  }, []);

  if (agents.length === 0) return null;

  return (
    <div className="agent-launcher">
      <div className="sidebar-section-title">Agents</div>
      <ul className="agent-list">
        {agents.map((agent) => (
          <li key={agent.id} className="agent-item" title={agent.description}>
            <span className="agent-name">{agent.name}</span>
            <button className="agent-launch-btn" onClick={() => onLaunch(agent)}>
              Launch
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
