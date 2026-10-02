use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;

use parking_lot::Mutex;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::broadcast;

pub struct TerminalHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    kill_tx: Sender<()>,
    /// Set by terminal_restart before killing the old process, so its
    /// reader thread doesn't emit `terminal://exit` for an id that has
    /// already been respawned (which would mark the live tab as exited).
    suppress_exit: Arc<AtomicBool>,
    pub title: Mutex<String>,
    /// Broadcasts this session's output for remote viewers (see remote.rs);
    /// has no effect on the local terminal, which uses the Tauri event above.
    pub output_tx: broadcast::Sender<String>,
    /// Remembered so terminal_restart can respawn the same shell/command
    /// under the same session id.
    spawn_cwd: Option<String>,
    spawn_initial_command: Option<String>,
}

#[derive(Default)]
pub struct TerminalRegistry(pub Mutex<HashMap<String, TerminalHandle>>);

impl TerminalRegistry {
    pub fn subscribe(&self, id: &str) -> Option<broadcast::Receiver<String>> {
        self.0.lock().get(id).map(|h| h.output_tx.subscribe())
    }

    pub fn list_sessions(&self) -> Vec<(String, String)> {
        self.0
            .lock()
            .iter()
            .map(|(id, h)| (id.clone(), h.title.lock().clone()))
            .collect()
    }
}

#[derive(Clone, Serialize)]
struct TerminalOutputEvent {
    id: String,
    data: String,
}

#[derive(Clone, Serialize)]
struct TerminalExitEvent {
    id: String,
    code: Option<i32>,
}

fn default_shell() -> (String, Vec<String>) {
    if cfg!(target_os = "windows") {
        (
            std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()),
            vec![],
        )
    } else {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
        (shell, vec!["-l".into()])
    }
}

/// Spawns a PTY running the login shell and inserts it into the registry
/// under `id` (which may be freshly generated or, for a restart, reused so
/// callers don't have to track a new session id).
pub(crate) fn spawn_into(
    app: &AppHandle,
    registry: &TerminalRegistry,
    id: String,
    cwd: Option<String>,
    cols: u16,
    rows: u16,
    initial_command: Option<String>,
    title: String,
) -> Result<(), String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let (shell, args) = default_shell();
    let mut cmd = CommandBuilder::new(shell);
    cmd.args(args);
    if let Some(dir) = &cwd {
        cmd.cwd(dir);
    }

    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| e.to_string())?;
    let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    if let Some(command) = &initial_command {
        let _ = writer.write_all(format!("{command}\r").as_bytes());
    }

    let (kill_tx, kill_rx) = channel::<()>();
    let (output_tx, _) = broadcast::channel(1024);

    let emit_app = app.clone();
    let emit_id = id.clone();
    let broadcast_tx = output_tx.clone();
    let suppress_exit = Arc::new(AtomicBool::new(false));
    let thread_suppress_exit = suppress_exit.clone();
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            if kill_rx.try_recv().is_ok() {
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = emit_app.emit(
                        "terminal://output",
                        TerminalOutputEvent {
                            id: emit_id.clone(),
                            data: data.clone(),
                        },
                    );
                    let _ = broadcast_tx.send(data);
                }
                Err(_) => break,
            }
        }
        if thread_suppress_exit.load(Ordering::SeqCst) {
            return;
        }
        let _ = emit_app.emit(
            "terminal://exit",
            TerminalExitEvent {
                id: emit_id.clone(),
                code: None,
            },
        );
    });

    registry.0.lock().insert(
        id,
        TerminalHandle {
            master: pair.master,
            writer,
            child,
            kill_tx,
            suppress_exit,
            title: Mutex::new(title),
            output_tx,
            spawn_cwd: cwd,
            spawn_initial_command: initial_command,
        },
    );

    Ok(())
}

#[tauri::command]
pub fn terminal_spawn(
    app: AppHandle,
    registry: State<TerminalRegistry>,
    cwd: Option<String>,
    cols: u16,
    rows: u16,
    initial_command: Option<String>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let title = format!("Terminal {}", &id[..8]);
    spawn_into(&app, &registry, id.clone(), cwd, cols, rows, initial_command, title)?;
    Ok(id)
}

#[tauri::command]
pub fn terminal_write(registry: State<TerminalRegistry>, id: String, data: String) -> Result<(), String> {
    let mut map = registry.0.lock();
    let handle = map.get_mut(&id).ok_or("terminal not found")?;
    handle
        .writer
        .write_all(data.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn terminal_resize(
    registry: State<TerminalRegistry>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let map = registry.0.lock();
    let handle = map.get(&id).ok_or("terminal not found")?;
    handle
        .master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn terminal_kill(registry: State<TerminalRegistry>, id: String) -> Result<(), String> {
    let mut map = registry.0.lock();
    if let Some(mut handle) = map.remove(&id) {
        let _ = handle.kill_tx.send(());
        let _ = handle.child.kill();
    }
    Ok(())
}

/// Lets the frontend sync a tab's display title into Rust state, so
/// remote viewers (which have no access to frontend-only tab state) see
/// something more useful than a raw session id.
#[tauri::command]
pub fn terminal_set_title(registry: State<TerminalRegistry>, id: String, title: String) -> Result<(), String> {
    let map = registry.0.lock();
    let handle = map.get(&id).ok_or("terminal not found")?;
    *handle.title.lock() = title;
    Ok(())
}

/// Sends Ctrl+C (a single fixed control byte, not arbitrary input) to
/// interrupt whatever is currently running in the session.
#[tauri::command]
pub fn terminal_cancel(registry: State<TerminalRegistry>, id: String) -> Result<(), String> {
    let mut map = registry.0.lock();
    let handle = map.get_mut(&id).ok_or("terminal not found")?;
    handle.writer.write_all(&[0x03]).map_err(|e| e.to_string())?;
    Ok(())
}

/// Kills and respawns a session under the same id, reusing its original
/// cwd/initial_command and title. Used for both the local UI and the
/// fixed-action remote control endpoint.
pub fn terminal_restart(
    app: &AppHandle,
    registry: &TerminalRegistry,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let (cwd, initial_command, title) = {
        let mut map = registry.0.lock();
        let mut handle = map.remove(&id).ok_or("terminal not found")?;
        handle.suppress_exit.store(true, Ordering::SeqCst);
        let _ = handle.kill_tx.send(());
        let _ = handle.child.kill();
        let title = handle.title.lock().clone();
        (handle.spawn_cwd.take(), handle.spawn_initial_command.take(), title)
    };
    spawn_into(app, registry, id, cwd, cols, rows, initial_command, title)
}

#[tauri::command]
pub fn terminal_restart_cmd(
    app: AppHandle,
    registry: State<TerminalRegistry>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    terminal_restart(&app, &registry, id, cols, rows)
}
