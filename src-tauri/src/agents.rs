use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

#[derive(Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    /// Shell command line typed into the terminal to launch this agent.
    pub command: String,
    pub description: String,
    /// Optional shell command whose exit code reports sign-in state; see
    /// check_agent_auth. None means auth status isn't tracked for this agent.
    #[serde(default)]
    pub auth_check_command: Option<String>,
}

fn builtin_agents() -> Vec<AgentDefinition> {
    vec![
        AgentDefinition {
            id: "claude-code".into(),
            name: "Claude Code".into(),
            command: "claude".into(),
            description: "Anthropic's Claude Code CLI".into(),
            auth_check_command: None,
        },
        AgentDefinition {
            id: "codex".into(),
            name: "Codex CLI".into(),
            command: "codex".into(),
            description: "OpenAI's Codex CLI".into(),
            auth_check_command: None,
        },
        AgentDefinition {
            id: "gemini".into(),
            name: "Gemini CLI".into(),
            command: "gemini".into(),
            description: "Google's Gemini CLI".into(),
            auth_check_command: None,
        },
        AgentDefinition {
            id: "aider".into(),
            name: "Aider".into(),
            command: "aider".into(),
            description: "Open-source AI pair programming CLI".into(),
            auth_check_command: None,
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

/// Runs a user-supplied shell command as an authentication probe and
/// reports whether it exited successfully (exit code 0), with a 10s
/// timeout so a hung/interactive command can't block the UI forever.
///
/// This is deliberately generic rather than hardcoded per built-in agent:
/// each CLI's real sign-in state lives in files/formats specific to that
/// tool that can change across versions, so instead of guessing at them
/// this lets the user supply their own check (e.g. `claude config get`
/// or any command that fails when unauthenticated) per custom agent.
#[tauri::command]
pub fn check_agent_auth(command: String, cwd: Option<String>) -> Result<bool, String> {
    let (shell, flag) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let mut cmd = Command::new(shell);
    cmd.arg(flag)
        .arg(&command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    match child.wait_timeout(Duration::from_secs(10)).map_err(|e| e.to_string())? {
        Some(status) => Ok(status.success()),
        None => {
            let _ = child.kill();
            Err("Auth check timed out".to_string())
        }
    }
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

    #[test]
    fn auth_check_reports_success_exit_code() {
        let cmd = if cfg!(target_os = "windows") { "exit 0" } else { "true" };
        assert!(check_agent_auth(cmd.to_string(), None).unwrap());
    }

    #[test]
    fn auth_check_reports_failure_exit_code() {
        let cmd = if cfg!(target_os = "windows") { "exit 1" } else { "false" };
        assert!(!check_agent_auth(cmd.to_string(), None).unwrap());
    }
}
