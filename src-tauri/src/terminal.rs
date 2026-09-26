use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Sender};
use std::thread;

use parking_lot::Mutex;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

pub struct TerminalHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    kill_tx: Sender<()>,
}

#[derive(Default)]
pub struct TerminalRegistry(pub Mutex<HashMap<String, TerminalHandle>>);

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
    if let Some(dir) = cwd {
        cmd.cwd(dir);
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| e.to_string())?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| e.to_string())?;
    let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    if let Some(command) = initial_command {
        let _ = writer.write_all(format!("{command}\r").as_bytes());
    }

    let (kill_tx, kill_rx) = channel::<()>();

    let emit_app = app.clone();
    let emit_id = id.clone();
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
                            data,
                        },
                    );
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
        id.clone(),
        TerminalHandle {
            master: pair.master,
            writer,
            child,
            kill_tx,
        },
    );

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
