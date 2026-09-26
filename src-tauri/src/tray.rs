use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};

const SHOW_ID: &str = "show";
const QUIT_ID: &str = "quit";

/// Builds the tray icon with a Show/Quit menu, so closing the main window
/// hides it rather than ending background terminal sessions — an agent
/// running in a background tab keeps running while the window is hidden.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let show_item = MenuItemBuilder::with_id(SHOW_ID, "Show Mihani").build(app)?;
    let quit_item = MenuItemBuilder::with_id(QUIT_ID, "Quit").build(app)?;
    let menu = MenuBuilder::new(app).item(&show_item).separator().item(&quit_item).build()?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?;

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("Mihani")
        .on_menu_event(move |app, event| match event.id.as_ref() {
            SHOW_ID => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            QUIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
