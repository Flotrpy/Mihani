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

/// Checks whether a command's binary is on PATH, using this process's
/// environment. Note this can under-report availability for tools only
/// added to PATH by shell startup files (e.g. via nvm), since those run
/// in the interactive shell rather than this process — a "not found"
/// result is a hint, not a guarantee the command will fail in-terminal.
#[tauri::command]
pub fn check_agent_installed(command: String) -> bool {
    let binary = command.split_whitespace().next().unwrap_or(&command);
    which::which(binary).is_ok()
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

    #[test]
    fn detects_a_known_installed_binary() {
        // `git` is guaranteed present in this project's dev/CI environment.
        assert!(check_agent_installed("git".to_string()));
    }

    #[test]
    fn reports_missing_binary_as_not_installed() {
        assert!(!check_agent_installed("mihani-definitely-not-a-real-binary".to_string()));
    }

    #[test]
    fn checks_only_the_first_word_of_a_command_with_arguments() {
        assert!(check_agent_installed("git status".to_string()));
    }
}
