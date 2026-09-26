use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "app.mihani";
const ACCOUNT: &str = "github-token";

fn token_entry() -> Result<Entry, String> {
    Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())
}

/// Stores the user's GitHub personal access token in the OS keychain
/// (Keychain on macOS, Credential Manager on Windows, Secret Service on
/// Linux) rather than in app config or plaintext on disk.
#[tauri::command]
pub fn github_set_token(token: String) -> Result<(), String> {
    token_entry()?.set_password(&token).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn github_clear_token() -> Result<(), String> {
    match token_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn get_token() -> Result<String, String> {
    token_entry()?
        .get_password()
        .map_err(|_| "No GitHub token configured".to_string())
}

#[derive(Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("Mihani")
        .build()
        .expect("failed to build HTTP client")
}

/// Validates the stored token by fetching the authenticated user, so PRs
/// and commits can be attributed to the user's own GitHub account rather
/// than a shared bot identity.
#[tauri::command]
pub fn github_whoami() -> Result<GitHubUser, String> {
    let token = get_token()?;
    let response = client()
        .get("https://api.github.com/user")
        .bearer_auth(&token)
        .send()
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("GitHub API error: {}", response.status()));
    }

    response.json::<GitHubUser>().map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct CreatedPullRequest {
    pub number: u64,
    pub html_url: String,
}

#[tauri::command]
pub fn github_create_pull_request(
    owner: String,
    repo: String,
    title: String,
    body: String,
    head: String,
    base: String,
) -> Result<CreatedPullRequest, String> {
    let token = get_token()?;

    let response = client()
        .post(format!("https://api.github.com/repos/{owner}/{repo}/pulls"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "title": title,
            "body": body,
            "head": head,
            "base": base,
        }))
        .send()
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().unwrap_or_default();
        return Err(format!("GitHub API error {status}: {text}"));
    }

    #[derive(Deserialize)]
    struct Response {
        number: u64,
        html_url: String,
    }
    let parsed: Response = response.json().map_err(|e| e.to_string())?;

    Ok(CreatedPullRequest {
        number: parsed.number,
        html_url: parsed.html_url,
    })
}
