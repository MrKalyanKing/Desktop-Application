use tauri::{AppHandle, menu::{MenuBuilder, MenuItemBuilder, Menu}};

pub fn build_tray_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let show = MenuItemBuilder::new("Show Widget").id("show").build(app)?;
    let hide = MenuItemBuilder::new("Hide Widget").id("hide").build(app)?;
    let settings = MenuItemBuilder::new("Open Settings").id("settings").build(app)?;
    let status = MenuItemBuilder::new("Check AI Status").id("status").build(app)?;
    let restart = MenuItemBuilder::new("Restart Gemini Connection").id("restart").build(app)?;
    let quit = MenuItemBuilder::new("Quit").id("quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show)
        .item(&hide)
        .separator()
        .item(&settings)
        .item(&status)
        .item(&restart)
        .separator()
        .item(&quit)
        .build()?;

    Ok(menu)
}
