use std::net::SocketAddr;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State as AxumState};
use axum::response::{Html, IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use parking_lot::Mutex;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tokio::net::TcpListener;

use crate::terminal::TerminalRegistry;

const TOKEN_CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

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
}

#[derive(Serialize)]
pub struct RemoteStatus {
    enabled: bool,
    token: String,
    port: u16,
    lan: bool,
}

#[tauri::command]
pub fn remote_status(state: tauri::State<RemoteState>) -> RemoteStatus {
    let inner = state.0.lock();
    RemoteStatus {
        enabled: inner.enabled,
        token: inner.token.clone(),
        port: inner.port,
        lan: inner.lan,
    }
}

#[tauri::command]
pub fn remote_regenerate_token(state: tauri::State<RemoteState>) -> String {
    let mut inner = state.0.lock();
    inner.token = generate_token();
    inner.token.clone()
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
            return Ok(RemoteStatus {
                enabled: true,
                token: inner.token.clone(),
                port: inner.port,
                lan: inner.lan,
            });
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
        let _ = axum::serve(listener, router)
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

    Ok(RemoteStatus {
        enabled: true,
        token,
        port,
        lan,
    })
}

#[tauri::command]
pub fn remote_disable(state: tauri::State<RemoteState>) {
    let mut inner = state.0.lock();
    if let Some(tx) = inner.shutdown_tx.take() {
        let _ = tx.send(());
    }
    inner.enabled = false;
}

#[derive(Clone)]
struct RouterState {
    app: AppHandle,
    token: String,
}

#[derive(Deserialize)]
struct AuthQuery {
    token: String,
}

fn build_router(app: AppHandle, token: String) -> Router {
    let state = RouterState { app, token };
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/", get(index_page))
        .route("/sessions", get(list_sessions))
        .route("/view/{id}", get(view_page))
        .route("/ws/{id}", get(ws_handler))
        .with_state(state)
}

fn check_auth(state: &RouterState, token: &str) -> bool {
    tokens_match(&state.token, token)
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

async fn list_sessions(
    AxumState(state): AxumState<RouterState>,
    Query(auth): Query<AuthQuery>,
) -> impl IntoResponse {
    if !check_auth(&state, &auth.token) {
        return (axum::http::StatusCode::UNAUTHORIZED, Json(Vec::<SessionInfo>::new())).into_response();
    }
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
    if !check_auth(&state, &auth.token) {
        return Html("<h1>Unauthorized</h1>".to_string());
    }
    let registry = state.app.state::<TerminalRegistry>();
    let items: String = registry
        .list_sessions()
        .into_iter()
        .map(|(id, title)| {
            format!(
                r#"<li><a href="/view/{id}?token={token}">{title}</a></li>"#,
                id = id,
                title = html_escape(&title),
                token = auth.token,
            )
        })
        .collect();
    Html(format!(
        r#"<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1">
        <title>Mihani Remote</title>
        <style>body{{font-family:sans-serif;background:#14101f;color:#ece7f9;padding:20px}}
        a{{color:#9b7cff}} li{{margin:8px 0}}</style></head>
        <body><h1>Mihani — active sessions</h1><ul>{items}</ul></body></html>"#
    ))
}

async fn view_page(Path(id): Path<String>, Query(auth): Query<AuthQuery>) -> impl IntoResponse {
    let token = auth.token;
    Html(format!(
        r#"<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1">
        <title>Mihani Remote</title>
        <style>body{{font-family:monospace;background:#0f0b1a;color:#ece7f9;margin:0;padding:10px}}
        pre{{white-space:pre-wrap;word-break:break-word}}</style></head>
        <body><pre id="out"></pre>
        <script>
          const out = document.getElementById('out');
          const proto = location.protocol === 'https:' ? 'wss' : 'ws';
          const ws = new WebSocket(proto + '://' + location.host + '/ws/{id}?token={token}');
          ws.onmessage = (e) => {{
            out.textContent += e.data;
            window.scrollTo(0, document.body.scrollHeight);
          }};
          ws.onclose = () => {{ out.textContent += '\n[connection closed]'; }};
        </script>
        </body></html>"#
    ))
}

async fn ws_handler(
    Path(id): Path<String>,
    Query(auth): Query<AuthQuery>,
    AxumState(state): AxumState<RouterState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if !check_auth(&state, &auth.token) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }
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
}
