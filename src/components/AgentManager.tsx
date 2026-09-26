import { useState } from "react";
import { useCustomAgents } from "../hooks/useCustomAgents";

export function AgentManager() {
  const { customAgents, addAgent, removeAgent } = useCustomAgents();
  const [newName, setNewName] = useState("");
  const [newCommand, setNewCommand] = useState("");

  const handleAdd = async () => {
    if (!newName.trim() || !newCommand.trim()) return;
    await addAgent({ name: newName.trim(), command: newCommand.trim(), description: "Custom agent" });
    setNewName("");
    setNewCommand("");
  };

  return (
    <div className="agent-manager">
      <p className="settings-hint">
        Custom agents run a shell command you specify. You'll be asked to confirm before the
        first run of each one.
      </p>
      <ul className="agent-manager-list">
        {customAgents.map((agent) => (
          <li key={agent.id} className="agent-manager-item">
            <div>
              <div className="agent-manager-name">{agent.name}</div>
              <code className="agent-manager-command">{agent.command}</code>
            </div>
            <button className="agent-remove-btn" onClick={() => removeAgent(agent.id)}>
              Remove
            </button>
          </li>
        ))}
        {customAgents.length === 0 && (
          <li className="settings-hint">No custom agents yet.</li>
        )}
      </ul>
      <div className="agent-add-form">
        <input placeholder="Name" value={newName} onChange={(e) => setNewName(e.target.value)} />
        <input
          placeholder="Shell command"
          value={newCommand}
          onChange={(e) => setNewCommand(e.target.value)}
        />
        <div className="agent-add-actions">
          <button onClick={handleAdd} disabled={!newName.trim() || !newCommand.trim()}>
            Add agent
          </button>
        </div>
      </div>
    </div>
  );
}
