use keepawake::KeepAwake;
use parking_lot::Mutex;

#[derive(Default)]
pub struct KeepAwakeState(pub Mutex<Option<KeepAwake>>);

/// Holds a system sleep-prevention assertion for as long as `state` contains
/// a handle. Backed by IOPMAssertionCreateWithName on macOS and
/// SetThreadExecutionState on Windows (both via the keepawake crate); the
/// assertion is released (system can sleep again) simply by dropping the
/// handle, which keepawake_disable/keepawake_enable's replacement do.
#[tauri::command]
pub fn keepawake_enable(state: tauri::State<KeepAwakeState>, reason: String) -> Result<(), String> {
    let handle = keepawake::Builder::default()
        .display(true)
        .idle(true)
        .sleep(true)
        .reason(reason)
        .app_name("Mihani")
        .app_reverse_domain("com.mihani.app")
        .create()
        .map_err(|e| e.to_string())?;

    *state.0.lock() = Some(handle);
    Ok(())
}

#[tauri::command]
pub fn keepawake_disable(state: tauri::State<KeepAwakeState>) {
    *state.0.lock() = None;
}

#[tauri::command]
pub fn keepawake_status(state: tauri::State<KeepAwakeState>) -> bool {
    state.0.lock().is_some()
}
