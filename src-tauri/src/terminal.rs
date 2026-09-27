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

/// Splits `bytes` into (decoded valid prefix, undecoded remainder). The
/// remainder is non-empty only when `bytes` ends with an incomplete
/// multi-byte UTF-8 sequence (at most 3 bytes); genuinely invalid bytes
/// (not just truncated) are lossily decoded instead, since buffering
/// wouldn't fix real corruption anyway.
fn split_valid_utf8(bytes: &[u8]) -> (String, Vec<u8>) {
    match std::str::from_utf8(bytes) {
        Ok(s) => (s.to_string(), Vec::new()),
        Err(e) if e.error_len().is_none() => {
            let valid_up_to = e.valid_up_to();
            let valid = std::str::from_utf8(&bytes[..valid_up_to])
                .expect("valid_up_to guarantees this prefix is valid UTF-8")
                .to_string();
            (valid, bytes[valid_up_to..].to_vec())
        }
        Err(_) => (String::from_utf8_lossy(bytes).into_owned(), Vec::new()),
    }
}

fn home_dir() -> Option<String> {
    if cfg!(target_os = "windows") {
        std::env::var("USERPROFILE").ok()
    } else {
        std::env::var("HOME").ok()
    }
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
// Each parameter is an independent, required piece of spawn configuration
// (not a group that wants its own type), and this is called from exactly
// two places (terminal_spawn, remote::spawn_and_announce), so a params
// struct would add indirection without a real second caller shape to serve.
#[allow(clippy::too_many_arguments)]
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
    // Without an explicit cwd, don't just inherit Mihani's own process cwd:
    // a macOS .app launched from Finder/Dock (rather than a terminal) often
    // starts with cwd "/", which would put a fresh shell somewhere useless
    // and surprising. Fall back to the user's home directory instead.
    if let Some(dir) = cwd.clone().or_else(home_dir) {
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
        // Bytes read but not yet decoded because they end mid-UTF-8-sequence
        // (a multi-byte character straddling a 4096-byte read boundary).
        // Carried into the next read rather than lossily replaced, so a
        // unicode symbol split across two reads doesn't render as a
        // replacement character — common in CLI agent output (spinners,
        // checkmarks, box-drawing).
        let mut leftover: Vec<u8> = Vec::new();
        loop {
            if kill_rx.try_recv().is_ok() {
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    leftover.extend_from_slice(&buf[..n]);
                    let (data, remainder) = split_valid_utf8(&leftover);
                    leftover = remainder;
                    if data.is_empty() {
                        continue;
                    }
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
    #[cfg(not(target_os = "windows"))]
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

    #[test]
    fn split_valid_utf8_passes_through_ascii() {
        let (data, remainder) = split_valid_utf8(b"hello world");
        assert_eq!(data, "hello world");
        assert!(remainder.is_empty());
    }

    #[test]
    fn split_valid_utf8_holds_back_a_sequence_split_across_reads() {
        // "✓" is U+2713, encoded as the 3 bytes [0xE2, 0x9C, 0x93]. Simulate
        // a read that stopped after the first byte of that sequence.
        let checkmark = "✓".as_bytes();
        assert_eq!(checkmark.len(), 3);

        let first_chunk = &checkmark[..1];
        let (data, remainder) = split_valid_utf8(first_chunk);
        assert_eq!(data, "");
        assert_eq!(remainder, checkmark[..1].to_vec());

        // Next read delivers the rest; prepend the held-back byte, as the
        // real reader loop does.
        let mut rejoined = remainder;
        rejoined.extend_from_slice(&checkmark[1..]);
        let (data, remainder) = split_valid_utf8(&rejoined);
        assert_eq!(data, "✓");
        assert!(remainder.is_empty());
    }

    #[test]
    fn split_valid_utf8_decodes_text_before_a_split_character() {
        let mut bytes = b"loading ".to_vec();
        bytes.extend_from_slice(&"✓".as_bytes()[..2]); // incomplete trailing char
        let (data, remainder) = split_valid_utf8(&bytes);
        assert_eq!(data, "loading ");
        assert_eq!(remainder.len(), 2);
    }

    #[test]
    fn split_valid_utf8_falls_back_to_lossy_for_genuinely_invalid_bytes() {
        let bytes = [b'a', 0xff, b'b'];
        let (data, remainder) = split_valid_utf8(&bytes);
        assert!(remainder.is_empty());
        assert!(data.contains('a') && data.contains('b'));
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn home_dir_reads_home_env_var() {
        let original = env::var("HOME").ok();

        unsafe { env::set_var("HOME", "/Users/example") };
        assert_eq!(home_dir(), Some("/Users/example".to_string()));

        unsafe { env::remove_var("HOME") };
        assert_eq!(home_dir(), None);

        match original {
            Some(v) => unsafe { env::set_var("HOME", v) },
            None => unsafe { env::remove_var("HOME") },
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
