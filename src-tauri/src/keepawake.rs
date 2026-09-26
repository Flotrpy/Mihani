use keepawake::KeepAwake;
use parking_lot::Mutex;

#[derive(Default)]
pub struct KeepAwakeState(pub Mutex<Option<KeepAwake>>);

#[tauri::command]
pub fn keepawake_enable(state: tauri::State<KeepAwakeState>, reason: String) -> Result<(), String> {
    let handle = keepawake::Builder::default()
        .display(true)
        .idle(true)
        .sleep(true)
        .reason(reason)
        .app_name("Mihani")
        .app_reverse_domain("app.mihani")
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
