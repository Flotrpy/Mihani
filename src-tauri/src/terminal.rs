use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Sender};
use std::thread;

use parking_lot::Mutex;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::broadcast;

pub struct TerminalHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    /// Only a killer, not the full Child: the Child itself is owned by a
    /// dedicated reaper thread (see spawn_into) that blocks on wait() for
    /// as long as the process runs, so it's reaped by the OS the instant it
    /// exits — whether from kill() here or the shell exiting on its own —
    /// instead of sitting as a zombie until Mihani itself quits.
    killer: Box<dyn ChildKiller + Send + Sync>,
    kill_tx: Sender<()>,
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

    let mut child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    drop(pair.slave);
    let killer = child.clone_killer();

    // Dedicated reaper: owns the Child and blocks on wait() until the
    // process exits, so the OS can reclaim it immediately rather than it
    // becoming a zombie. Runs independently of the output-reading thread
    // below and of terminal_kill/terminal_restart, which only ever touch
    // `killer`.
    thread::spawn(move || {
        let _ = child.wait();
    });

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
            killer,
            kill_tx,
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
        let _ = handle.killer.kill();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Combined into one test (rather than two) because both mutate the
    // process-global SHELL env var, which would race under cargo test's
    // default parallel execution otherwise. Exercises the non-Windows
    // branch of default_shell(), shared by macOS and Linux (login shell
    // via $SHELL) — this covers macOS's actual runtime behavior even
    // though CI runs it on whatever OS the job happens to be.
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn default_shell_on_unix() {
        let original = env::var("SHELL").ok();

        unsafe { env::set_var("SHELL", "/bin/zsh") };
        let (shell, args) = default_shell();
        assert_eq!(shell, "/bin/zsh");
        assert_eq!(args, vec!["-l".to_string()]);

        unsafe { env::remove_var("SHELL") };
        let (shell, args) = default_shell();
        assert_eq!(shell, "/bin/bash");
        assert_eq!(args, vec!["-l".to_string()]);

        match original {
            Some(v) => unsafe { env::set_var("SHELL", v) },
            None => unsafe { env::remove_var("SHELL") },
        }
    }
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
        let _ = handle.kill_tx.send(());
        let _ = handle.killer.kill();
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
