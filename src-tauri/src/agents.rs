use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    /// Shell command line typed into the terminal to launch this agent.
    pub command: String,
    pub description: String,
}

fn builtin_agents() -> Vec<AgentDefinition> {
    vec![
        AgentDefinition {
            id: "claude-code".into(),
            name: "Claude Code".into(),
            command: "claude".into(),
            description: "Anthropic's Claude Code CLI".into(),
        },
        AgentDefinition {
            id: "codex".into(),
            name: "Codex CLI".into(),
            command: "codex".into(),
            description: "OpenAI's Codex CLI".into(),
        },
        AgentDefinition {
            id: "gemini".into(),
            name: "Gemini CLI".into(),
            command: "gemini".into(),
            description: "Google's Gemini CLI".into(),
        },
        AgentDefinition {
            id: "aider".into(),
            name: "Aider".into(),
            command: "aider".into(),
            description: "Open-source AI pair programming CLI".into(),
        },
    ]
}

#[tauri::command]
pub fn list_agents() -> Vec<AgentDefinition> {
    builtin_agents()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_agents_have_unique_ids() {
        let agents = builtin_agents();
        let mut ids: Vec<_> = agents.iter().map(|a| a.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), agents.len());
    }

    #[test]
    fn builtin_agents_is_non_empty() {
        assert!(!builtin_agents().is_empty());
    }
}
