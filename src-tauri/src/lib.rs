mod agents;
mod app_menu;
mod git;
mod github;
mod keepawake;
mod remote;
mod terminal;
mod tray;

use keepawake::KeepAwakeState;
use remote::RemoteState;
#[cfg(target_os = "macos")]
use tauri::Manager;
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
        .manage(RemoteState::default())
        .setup(|app| {
            let menu = app_menu::build(app.handle())?;
            app.set_menu(menu)?;

            // On Linux, creating the tray icon dlopen()s libayatana-appindicator3
            // (or the older libappindicator3) and panics — rather than returning
            // an Err — if neither is installed on the system. That's a real gap
            // on minimal desktop environments, so don't let it take the whole
            // app down: the tray icon is a convenience, not a requirement.
            let handle = app.handle().clone();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                tray::build(&handle)
            }));
            if let Err(panic) = result {
                let message = panic
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown panic".to_string());
                eprintln!("Mihani: tray icon unavailable, continuing without it: {message}");
            }

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
            terminal::terminal_set_title,
            terminal::terminal_cancel,
            terminal::terminal_restart_cmd,
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
            remote::remote_status,
            remote::remote_enable,
            remote::remote_disable,
            remote::remote_regenerate_token,
            remote::remote_set_context,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // Closing the main window hides it instead of quitting (see the
            // CloseRequested handler above), so background terminal sessions
            // keep running. On macOS that leaves the app backgrounded with no
            // visible window and no dock menu action of its own — clicking the
            // dock icon sends Reopen, which the OS expects to bring a window
            // back. Without handling it, the app would be effectively stuck
            // with no way to reach it again short of quitting from the tray.
            // Reopen only exists in tauri's RunEvent on macOS.
            #[cfg(target_os = "macos")]
            {
                if let tauri::RunEvent::Reopen { has_visible_windows, .. } = event {
                    if !has_visible_windows {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (app_handle, event);
            }
        });
}
