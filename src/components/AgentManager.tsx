import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useCustomAgents } from "../hooks/useCustomAgents";

export function AgentManager() {
  const { customAgents, addAgent, removeAgent } = useCustomAgents();
  const [newName, setNewName] = useState("");
  const [newCommand, setNewCommand] = useState("");
  const [newAuthCheck, setNewAuthCheck] = useState("");
  const [authStatus, setAuthStatus] = useState<Record<string, "checking" | boolean>>({});

  const handleAdd = async () => {
    if (!newName.trim() || !newCommand.trim()) return;
    await addAgent({
      name: newName.trim(),
      command: newCommand.trim(),
      description: "Custom agent",
      auth_check_command: newAuthCheck.trim() || null,
    });
    setNewName("");
    setNewCommand("");
    setNewAuthCheck("");
  };

  const runAuthCheck = (id: string, command: string) => {
    setAuthStatus((prev) => ({ ...prev, [id]: "checking" }));
    invoke<boolean>("check_agent_auth", { command, cwd: null })
      .then((ok) => setAuthStatus((prev) => ({ ...prev, [id]: ok })))
      .catch(() => setAuthStatus((prev) => ({ ...prev, [id]: false })));
  };

  useEffect(() => {
    for (const agent of customAgents) {
      if (agent.auth_check_command) runAuthCheck(agent.id, agent.auth_check_command);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [customAgents]);

  return (
    <div className="agent-manager">
      <p className="settings-hint">
        Custom agents run a shell command you specify. You'll be asked to confirm before the
        first run of each one. An optional auth check command reports sign-in status (exit code 0
        = signed in) — for example a CLI's own "whoami" or config-check subcommand.
      </p>
      <ul className="agent-manager-list">
        {customAgents.map((agent) => (
          <li key={agent.id} className="agent-manager-item">
            <div>
              <div className="agent-manager-name">{agent.name}</div>
              <code className="agent-manager-command">{agent.command}</code>
              {agent.auth_check_command && (
                <div className="agent-manager-auth">
                  {authStatus[agent.id] === "checking" && "Checking auth…"}
                  {authStatus[agent.id] === true && (
                    <span className="agent-auth-ok">Signed in</span>
                  )}
                  {authStatus[agent.id] === false && (
                    <span className="agent-auth-fail">Not signed in</span>
                  )}
                </div>
              )}
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
        <input
          placeholder="Auth check command (optional)"
          value={newAuthCheck}
          onChange={(e) => setNewAuthCheck(e.target.value)}
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
