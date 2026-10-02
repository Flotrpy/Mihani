use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, Request, State as AxumState};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use parking_lot::Mutex;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::TcpListener;

use crate::agents::list_agents;
use crate::git::{git_diff, git_status};
use crate::terminal::{terminal_cancel, terminal_kill, terminal_restart, TerminalRegistry};

const TOKEN_CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const DEFAULT_COLS: u16 = 100;
const DEFAULT_ROWS: u16 = 30;

fn generate_token() -> String {
    let mut rng = rand::rng();
    (0..8)
        .map(|_| TOKEN_CHARS[rng.random_range(0..TOKEN_CHARS.len())] as char)
        .collect()
}

#[derive(Default)]
pub struct RemoteState(pub Mutex<RemoteInner>);

#[derive(Default)]
pub struct RemoteInner {
    pub enabled: bool,
    pub token: String,
    pub port: u16,
    pub lan: bool,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    /// Synced from the frontend so fixed remote actions (run tests, commit
    /// requests) know what they apply to, without the remote client ever
    /// supplying a path or command itself.
    pub workspace_path: Option<String>,
    pub test_command: Option<String>,
}

#[derive(Serialize)]
pub struct RemoteStatus {
    enabled: bool,
    token: String,
    port: u16,
    lan: bool,
}

fn status_from(inner: &RemoteInner) -> RemoteStatus {
    RemoteStatus {
        enabled: inner.enabled,
        token: inner.token.clone(),
        port: inner.port,
        lan: inner.lan,
    }
}

#[tauri::command]
pub fn remote_status(state: tauri::State<RemoteState>) -> RemoteStatus {
    status_from(&state.0.lock())
}

#[tauri::command]
pub fn remote_regenerate_token(state: tauri::State<RemoteState>) -> String {
    let mut inner = state.0.lock();
    inner.token = generate_token();
    inner.token.clone()
}

/// Lets the frontend tell the remote server which workspace/test command
/// fixed actions ("run tests", "request commit & push") should apply to.
/// The remote client never supplies these itself.
#[tauri::command]
pub fn remote_set_context(
    state: tauri::State<RemoteState>,
    workspace_path: Option<String>,
    test_command: Option<String>,
) {
    let mut inner = state.0.lock();
    inner.workspace_path = workspace_path;
    inner.test_command = test_command;
}

#[tauri::command]
pub async fn remote_enable(
    app: AppHandle,
    state: tauri::State<'_, RemoteState>,
    lan: bool,
) -> Result<RemoteStatus, String> {
    {
        let inner = state.0.lock();
        if inner.enabled {
            return Ok(status_from(&inner));
        }
    }

    let bind_addr: SocketAddr = if lan {
        "0.0.0.0:0".parse().unwrap()
    } else {
        "127.0.0.1:0".parse().unwrap()
    };
    let listener = TcpListener::bind(bind_addr).await.map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let token = {
        let mut inner = state.0.lock();
        if inner.token.is_empty() {
            inner.token = generate_token();
        }
        inner.token.clone()
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let router = build_router(app.clone(), token.clone());

    tauri::async_runtime::spawn(async move {
        let _ = axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        })
        .await;
    });

    let mut inner = state.0.lock();
    inner.enabled = true;
    inner.port = port;
    inner.lan = lan;
    inner.shutdown_tx = Some(shutdown_tx);

    Ok(status_from(&inner))
}

#[tauri::command]
pub fn remote_disable(state: tauri::State<RemoteState>) {
    let mut inner = state.0.lock();
    if let Some(tx) = inner.shutdown_tx.take() {
        let _ = tx.send(());
    }
    inner.enabled = false;
}

const MAX_FAILURES_BEFORE_LOCKOUT: u32 = 10;
const LOCKOUT_DURATION: Duration = Duration::from_secs(60);

struct RateEntry {
    failures: u32,
    locked_until: Option<Instant>,
}

#[derive(Clone, Default)]
struct RateLimiter(Arc<Mutex<HashMap<IpAddr, RateEntry>>>);

impl RateLimiter {
    fn is_locked(&self, ip: IpAddr) -> bool {
        self.0
            .lock()
            .get(&ip)
            .and_then(|e| e.locked_until)
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }

    fn record_failure(&self, ip: IpAddr) {
        let mut map = self.0.lock();
        let entry = map.entry(ip).or_insert(RateEntry { failures: 0, locked_until: None });
        entry.failures += 1;
        if entry.failures >= MAX_FAILURES_BEFORE_LOCKOUT {
            entry.locked_until = Some(Instant::now() + LOCKOUT_DURATION);
            entry.failures = 0;
        }
    }

    fn record_success(&self, ip: IpAddr) {
        self.0.lock().remove(&ip);
    }
}

#[derive(Clone)]
struct RouterState {
    app: AppHandle,
    token: String,
    rate_limiter: RateLimiter,
}

/// Single auth gate for every route except /health: checked here, before
/// any handler runs, so a wrong token never reaches handler logic at all.
/// Also tracks failures per source IP and locks out an IP for
/// LOCKOUT_DURATION after MAX_FAILURES_BEFORE_LOCKOUT wrong attempts —
/// defense in depth against brute-forcing the token, on top of the token
/// space itself (32^8) already making that impractical.
async fn auth_gate(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<RouterState>,
    request: Request,
    next: Next,
) -> Response {
    if request.uri().path() == "/health" {
        return next.run(request).await;
    }

    let ip = addr.ip();
    if state.rate_limiter.is_locked(ip) {
        return (
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "too many failed attempts; try again in a minute",
        )
            .into_response();
    }

    let token_ok = request
        .uri()
        .query()
        .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("token=")))
        .map(|t| tokens_match(&state.token, t))
        .unwrap_or(false);

    if !token_ok {
        state.rate_limiter.record_failure(ip);
        return (axum::http::StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }

    state.rate_limiter.record_success(ip);
    next.run(request).await
}

#[derive(Deserialize)]
struct AuthQuery {
    token: String,
}

fn build_router(app: AppHandle, token: String) -> Router {
    let state = RouterState { app, token, rate_limiter: RateLimiter::default() };
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/", get(index_page))
        .route("/sessions", get(list_sessions))
        .route("/view/{id}", get(view_page))
        .route("/ws/{id}", get(ws_handler))
        .route("/action", post(action_handler))
        .route("/git/status", get(git_status_handler))
        .route("/git/diff", get(git_diff_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_gate))
        .with_state(state)
}

fn tokens_match(expected: &str, actual: &str) -> bool {
    // Simple constant-time-ish comparison; low-stakes local secret, not a
    // web-scale credential, but avoid the obvious short-circuit compare.
    let a = expected.as_bytes();
    let b = actual.as_bytes();
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[derive(Serialize)]
struct SessionInfo {
    id: String,
    title: String,
}

async fn list_sessions(AxumState(state): AxumState<RouterState>) -> impl IntoResponse {
    let registry = state.app.state::<TerminalRegistry>();
    let sessions: Vec<SessionInfo> = registry
        .list_sessions()
        .into_iter()
        .map(|(id, title)| SessionInfo { id, title })
        .collect();
    Json(sessions).into_response()
}

async fn index_page(
    AxumState(state): AxumState<RouterState>,
    Query(auth): Query<AuthQuery>,
) -> impl IntoResponse {
    let registry = state.app.state::<TerminalRegistry>();
    let items: String = registry
        .list_sessions()
        .into_iter()
        .map(|(id, title)| {
            format!(
                r#"<li><a href="/view/{id}?token={token}">{title}</a>
                <button onclick="sessionAction('{id}','cancel')">Cancel</button>
                <button onclick="sessionAction('{id}','restart')">Restart</button>
                <button onclick="sessionAction('{id}','stop')">Stop</button></li>"#,
                id = id,
                title = html_escape(&title),
                token = auth.token,
            )
        })
        .collect();

    let agent_buttons: String = list_agents()
        .into_iter()
        .map(|agent| {
            format!(
                r#"<button onclick="startAgent('{id}')">{name}</button>"#,
                id = agent.id,
                name = html_escape(&agent.name),
            )
        })
        .collect();
    Html(format!(
        r#"<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1">
        <title>Mihani Remote</title>
        <style>body{{font-family:sans-serif;background:#14101f;color:#ece7f9;padding:20px}}
        a{{color:#9b7cff}} li{{margin:8px 0}} button{{background:#7c5cff;color:#fff;border:none;
        border-radius:6px;padding:8px 14px;margin:4px 6px 4px 0;cursor:pointer}}
        pre{{white-space:pre-wrap;word-break:break-word;background:#0f0b1a;padding:10px;border-radius:6px}}
        .file{{cursor:pointer;color:#9b7cff}} #gitOut{{margin-top:10px}}</style></head>
        <body><h1>Mihani — active sessions</h1><ul>{items}</ul>
        <h2>Start an agent</h2>
        {agent_buttons}
        <h2>Actions</h2>
        <button onclick="fetch('/action?token={token}',{{method:'POST',headers:{{'content-type':'application/json'}},body:JSON.stringify({{action:'run_tests'}})}})">Run tests</button>
        <button onclick="fetch('/action?token={token}',{{method:'POST',headers:{{'content-type':'application/json'}},body:JSON.stringify({{action:'request_commit_push'}})}})">Request commit &amp; push</button>
        <h2>Git</h2>
        <button onclick="loadStatus()">Refresh status</button>
        <div id="gitOut"></div>
        <script>
          function sessionAction(id, action) {{
            fetch('/action?token={token}', {{
              method: 'POST',
              headers: {{'content-type': 'application/json'}},
              body: JSON.stringify({{action, session_id: id}}),
            }}).then(() => setTimeout(() => location.reload(), 300));
          }}
          function startAgent(agentId) {{
            fetch('/action?token={token}', {{
              method: 'POST',
              headers: {{'content-type': 'application/json'}},
              body: JSON.stringify({{action: 'start_agent', agent_id: agentId}}),
            }}).then(() => setTimeout(() => location.reload(), 500));
          }}
          async function loadStatus() {{
            const out = document.getElementById('gitOut');
            out.textContent = 'Loading…';
            const res = await fetch('/git/status?token={token}');
            if (!res.ok) {{ out.textContent = 'Error: ' + await res.text(); return; }}
            const status = await res.json();
            // Branch names and file paths come from the repository, so they
            // are inserted as text nodes, never parsed as HTML.
            out.replaceChildren();
            if (!status.files.length) {{
              const p = document.createElement('p');
              p.textContent = 'Working tree clean';
              out.appendChild(p);
              return;
            }}
            const branch = document.createElement('p');
            branch.textContent = 'Branch: ' + status.branch;
            const list = document.createElement('ul');
            for (const f of status.files) {{
              const li = document.createElement('li');
              li.className = 'file';
              li.textContent = '[' + f.status + '] ' + f.path;
              li.addEventListener('click', () => loadDiff(f.path));
              list.appendChild(li);
            }}
            const diff = document.createElement('pre');
            diff.id = 'diffOut';
            out.append(branch, list, diff);
          }}
          async function loadDiff(file) {{
            const res = await fetch('/git/diff?token={token}&file=' + encodeURIComponent(file));
            document.getElementById('diffOut').textContent = await res.text();
          }}
        </script>
        </body></html>"#,
        items = items,
        agent_buttons = agent_buttons,
        token = auth.token,
    ))
}

async fn view_page(
    Path(id): Path<String>,
    AxumState(state): AxumState<RouterState>,
    Query(auth): Query<AuthQuery>,
) -> Response {
    // `id` is interpolated into inline JS below, so only accept ids of
    // sessions that actually exist (generated UUIDs) rather than echoing
    // an arbitrary path segment back into the page.
    let known = state
        .app
        .state::<TerminalRegistry>()
        .list_sessions()
        .iter()
        .any(|(session_id, _)| *session_id == id);
    if !known || !is_safe_session_id(&id) {
        return (axum::http::StatusCode::NOT_FOUND, "session not found").into_response();
    }
    let token = auth.token;
    Html(format!(
        r#"<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1">
        <title>Mihani Remote</title>
        <style>body{{font-family:monospace;background:#0f0b1a;color:#ece7f9;margin:0;padding:10px}}
        pre{{white-space:pre-wrap;word-break:break-word}}
        .bar{{position:sticky;top:0;display:flex;gap:6px;padding:8px 0;background:#0f0b1a}}
        button{{background:#221a3a;color:#ece7f9;border:1px solid #33285a;border-radius:6px;
        padding:6px 10px;cursor:pointer}}</style></head>
        <body>
        <div class="bar">
          <button onclick="act('cancel')">Cancel (Ctrl+C)</button>
          <button onclick="act('restart')">Restart</button>
          <button onclick="act('stop')">Stop</button>
        </div>
        <pre id="out"></pre>
        <script>
          const out = document.getElementById('out');
          const proto = location.protocol === 'https:' ? 'wss' : 'ws';
          const ws = new WebSocket(proto + '://' + location.host + '/ws/{id}?token={token}');
          ws.onmessage = (e) => {{
            out.textContent += e.data;
            window.scrollTo(0, document.body.scrollHeight);
          }};
          ws.onclose = () => {{ out.textContent += '\n[connection closed]'; }};
          function act(action) {{
            fetch('/action?token={token}', {{
              method: 'POST',
              headers: {{'content-type': 'application/json'}},
              body: JSON.stringify({{action, session_id: '{id}'}}),
            }});
          }}
        </script>
        </body></html>"#
    ))
    .into_response()
}

async fn ws_handler(
    Path(id): Path<String>,
    AxumState(state): AxumState<RouterState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.app, id))
}

async fn handle_socket(mut socket: WebSocket, app: AppHandle, id: String) {
    let registry = app.state::<TerminalRegistry>();
    let mut rx = match registry.subscribe(&id) {
        Some(rx) => rx,
        None => {
            let _ = socket.send(Message::Text("[session not found]".into())).await;
            return;
        }
    };
    drop(registry);

    // View-only: this loop only ever sends terminal output to the client.
    // Any inbound message from the client is ignored — this websocket is
    // not wired to terminal_write, and the fixed action set below (which
    // is the only way a remote client can affect a session) has no
    // "send text" action.
    loop {
        match rx.recv().await {
            Ok(data) => {
                if socket.send(Message::Text(data.into())).await.is_err() {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}

#[derive(Clone, Serialize)]
struct SessionStartedEvent {
    id: String,
    title: String,
}

#[derive(Clone, Serialize)]
struct CommitRequestEvent {
    path: String,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum RemoteAction {
    Cancel { session_id: String },
    Stop { session_id: String },
    Restart { session_id: String },
    StartAgent { agent_id: String },
    RunTests,
    RequestCommitPush,
}

async fn action_handler(
    AxumState(state): AxumState<RouterState>,
    Json(action): Json<RemoteAction>,
) -> impl IntoResponse {
    let app = state.app;
    match action {
        RemoteAction::Cancel { session_id } => {
            let registry = app.state::<TerminalRegistry>();
            match terminal_cancel(registry, session_id) {
                Ok(()) => axum::http::StatusCode::OK.into_response(),
                Err(e) => (axum::http::StatusCode::NOT_FOUND, e).into_response(),
            }
        }
        RemoteAction::Stop { session_id } => {
            let registry = app.state::<TerminalRegistry>();
            match terminal_kill(registry, session_id) {
                Ok(()) => axum::http::StatusCode::OK.into_response(),
                Err(e) => (axum::http::StatusCode::NOT_FOUND, e).into_response(),
            }
        }
        RemoteAction::Restart { session_id } => {
            let registry = app.state::<TerminalRegistry>();
            match terminal_restart(&app, &registry, session_id, DEFAULT_COLS, DEFAULT_ROWS) {
                Ok(()) => axum::http::StatusCode::OK.into_response(),
                Err(e) => (axum::http::StatusCode::NOT_FOUND, e).into_response(),
            }
        }
        RemoteAction::StartAgent { agent_id } => {
            // Only built-in agents (a fixed, curated list) can be started
            // remotely — never a user-defined custom agent, since those
            // are arbitrary commands the remote client must not be able
            // to trigger.
            let Some(agent) = list_agents().into_iter().find(|a| a.id == agent_id) else {
                return (axum::http::StatusCode::NOT_FOUND, "unknown agent").into_response();
            };
            let cwd = app.state::<RemoteState>().0.lock().workspace_path.clone();
            match spawn_and_announce(&app, cwd, Some(agent.command), agent.name) {
                Ok(()) => axum::http::StatusCode::OK.into_response(),
                Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
            }
        }
        RemoteAction::RunTests => {
            let (cwd, test_command) = {
                let remote_state = app.state::<RemoteState>();
                let inner = remote_state.0.lock();
                (inner.workspace_path.clone(), inner.test_command.clone())
            };
            let Some(command) = test_command else {
                return (axum::http::StatusCode::BAD_REQUEST, "no test command configured").into_response();
            };
            match spawn_and_announce(&app, cwd, Some(command), "Tests".to_string()) {
                Ok(()) => axum::http::StatusCode::OK.into_response(),
                Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
            }
        }
        RemoteAction::RequestCommitPush => {
            let path = app.state::<RemoteState>().0.lock().workspace_path.clone();
            let Some(path) = path else {
                return (axum::http::StatusCode::BAD_REQUEST, "no workspace open").into_response();
            };
            // Never commits/pushes directly: this only asks the local app
            // to show its existing commit UI, which the user must confirm.
            let _ = app.emit("remote://commit-request", CommitRequestEvent { path });
            axum::http::StatusCode::OK.into_response()
        }
    }
}

fn spawn_and_announce(
    app: &AppHandle,
    cwd: Option<String>,
    initial_command: Option<String>,
    title: String,
) -> Result<(), String> {
    let id = uuid::Uuid::new_v4().to_string();
    let registry = app.state::<TerminalRegistry>();
    crate::terminal::spawn_into(
        app,
        &registry,
        id.clone(),
        cwd,
        DEFAULT_COLS,
        DEFAULT_ROWS,
        initial_command,
        title.clone(),
    )?;
    let _ = app.emit("remote://session-started", SessionStartedEvent { id, title });
    Ok(())
}

async fn git_status_handler(AxumState(state): AxumState<RouterState>) -> impl IntoResponse {
    let path = state.app.state::<RemoteState>().0.lock().workspace_path.clone();
    let Some(path) = path else {
        return (axum::http::StatusCode::BAD_REQUEST, "no workspace open").into_response();
    };
    match git_status(path) {
        Ok(status) => Json(status).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

#[derive(Deserialize)]
struct DiffQuery {
    file: String,
}

async fn git_diff_handler(
    Query(diff_query): Query<DiffQuery>,
    AxumState(state): AxumState<RouterState>,
) -> impl IntoResponse {
    let path = state.app.state::<RemoteState>().0.lock().workspace_path.clone();
    let Some(path) = path else {
        return (axum::http::StatusCode::BAD_REQUEST, "no workspace open").into_response();
    };
    match git_diff(path, diff_query.file) {
        Ok(diff) => diff.into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

/// Session ids are generated UUIDs; anything outside that alphabet is
/// rejected before being placed in a page.
fn is_safe_session_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_matching_token() {
        assert!(tokens_match("ABCD1234", "ABCD1234"));
    }

    #[test]
    fn rejects_wrong_token() {
        assert!(!tokens_match("ABCD1234", "WRONGTOK"));
    }

    #[test]
    fn rejects_different_length_token() {
        assert!(!tokens_match("ABCD1234", "AB"));
    }

    #[test]
    fn generated_tokens_have_expected_length_and_alphabet() {
        let token = generate_token();
        assert_eq!(token.len(), 8);
        assert!(token.chars().all(|c| TOKEN_CHARS.contains(&(c as u8))));
    }

    #[test]
    fn html_escape_neutralizes_tags() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
    }

    #[test]
    fn session_id_validation_rejects_script_injection() {
        assert!(is_safe_session_id("3f2b6c1e-9a4d-4e8b-b1c2-0d9e8f7a6b5c"));
        assert!(!is_safe_session_id(""));
        assert!(!is_safe_session_id("x');alert(1);('"));
        assert!(!is_safe_session_id("<script>"));
    }

    #[test]
    fn remote_action_deserializes_fixed_variants_only() {
        let cancel: RemoteAction =
            serde_json::from_str(r#"{"action":"cancel","session_id":"abc"}"#).unwrap();
        assert!(matches!(cancel, RemoteAction::Cancel { session_id } if session_id == "abc"));

        let run_tests: RemoteAction = serde_json::from_str(r#"{"action":"run_tests"}"#).unwrap();
        assert!(matches!(run_tests, RemoteAction::RunTests));

        // No "send_text" / "write" / freeform-input variant exists at all,
        // so it can't be deserialized even if a client tries to send one.
        let attempt: Result<RemoteAction, _> =
            serde_json::from_str(r#"{"action":"send_text","data":"rm -rf /"}"#);
        assert!(attempt.is_err());
    }
}
