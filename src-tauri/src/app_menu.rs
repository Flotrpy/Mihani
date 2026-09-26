use tauri::menu::{AboutMetadataBuilder, MenuBuilder, SubmenuBuilder};
use tauri::{AppHandle, Runtime};

/// Builds the native application menu (macOS menu bar / Windows menu bar),
/// with the standard Edit and Window submenus so copy/paste/undo and
/// window management work through the OS's usual shortcuts, not just
/// inside the embedded terminal.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    let about = AboutMetadataBuilder::new()
        .name(Some("Mihani"))
        .version(Some(env!("CARGO_PKG_VERSION")))
        .build();

    let app_menu = SubmenuBuilder::new(app, "Mihani")
        .about(Some(about))
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .close_window()
        .build()?;

    MenuBuilder::new(app)
        .item(&app_menu)
        .item(&edit_menu)
        .item(&window_menu)
        .build()
}
