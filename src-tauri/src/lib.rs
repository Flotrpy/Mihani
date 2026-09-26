mod agents;
mod git;
mod terminal;

use terminal::TerminalRegistry;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(TerminalRegistry::default())
        .invoke_handler(tauri::generate_handler![
            terminal::terminal_spawn,
            terminal::terminal_write,
            terminal::terminal_resize,
            terminal::terminal_kill,
            git::git_status,
            git::git_commit,
            git::git_push,
            git::git_pull,
            agents::list_agents,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
