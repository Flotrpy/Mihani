mod agents;
mod app_menu;
mod git;
mod github;
mod keepawake;
mod terminal;
mod tray;

use keepawake::KeepAwakeState;
use tauri::WindowEvent;
use terminal::TerminalRegistry;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .manage(TerminalRegistry::default())
        .manage(KeepAwakeState::default())
        .setup(|app| {
            let menu = app_menu::build(app.handle())?;
            app.set_menu(menu)?;
            tray::build(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            terminal::terminal_spawn,
            terminal::terminal_write,
            terminal::terminal_resize,
            terminal::terminal_kill,
            git::git_status,
            git::git_diff,
            git::git_commit,
            git::git_push,
            git::git_pull,
            git::git_list_branches,
            git::git_checkout_branch,
            git::git_create_branch,
            git::git_remote_info,
            git::git_discard_file,
            git::git_log,
            agents::list_agents,
            agents::check_agent_installed,
            agents::check_agent_auth,
            keepawake::keepawake_enable,
            keepawake::keepawake_disable,
            keepawake::keepawake_status,
            github::github_set_token,
            github::github_clear_token,
            github::github_whoami,
            github::github_create_pull_request,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
