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
}

export function AgentLauncher({ onLaunch }: AgentLauncherProps) {
  const [builtins, setBuiltins] = useState<AgentDefinition[]>([]);
  const { customAgents, trustedIds, addAgent, removeAgent, trustAgent } = useCustomAgents();
  const [pendingTrust, setPendingTrust] = useState<AgentDefinition | null>(null);
  const [showAddForm, setShowAddForm] = useState(false);
  const [newName, setNewName] = useState("");
  const [newCommand, setNewCommand] = useState("");

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

  const handleAddAgent = async () => {
    if (!newName.trim() || !newCommand.trim()) return;
    await addAgent({ name: newName.trim(), command: newCommand.trim(), description: "Custom agent" });
    setNewName("");
    setNewCommand("");
    setShowAddForm(false);
  };

  const allAgents = [...builtins, ...customAgents];
  if (allAgents.length === 0 && !showAddForm) return null;

  return (
    <div className="agent-launcher">
      <div className="sidebar-section-title">Agents</div>
      <ul className="agent-list">
        {allAgents.map((agent) => (
          <li key={agent.id} className="agent-item" title={agent.description}>
            <span className="agent-name">
              {agent.name}
              {isCustom(agent) && <span className="agent-custom-badge">custom</span>}
            </span>
            <span className="agent-item-actions">
              <button className="agent-launch-btn" onClick={() => handleLaunchClick(agent)}>
                Launch
              </button>
              {isCustom(agent) && (
                <button className="agent-remove-btn" onClick={() => removeAgent(agent.id)}>
                  ×
                </button>
              )}
            </span>
          </li>
        ))}
      </ul>

      {showAddForm ? (
        <div className="agent-add-form">
          <input
            placeholder="Name"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
          <input
            placeholder="Shell command"
            value={newCommand}
            onChange={(e) => setNewCommand(e.target.value)}
          />
          <div className="agent-add-actions">
            <button onClick={handleAddAgent}>Add</button>
            <button onClick={() => setShowAddForm(false)}>Cancel</button>
          </div>
        </div>
      ) : (
        <button className="agent-add-btn" onClick={() => setShowAddForm(true)}>
          + Add custom agent
        </button>
      )}

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
