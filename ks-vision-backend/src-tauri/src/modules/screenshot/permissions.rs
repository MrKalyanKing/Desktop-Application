#[allow(dead_code)]
pub fn check_screenshot_permissions() -> Result<(), String> {
    let screens = screenshots::Screen::all().map_err(|e| e.to_string())?;
    if screens.is_empty() {
        return Err("No active displays found".to_string());
    }
    Ok(())
}
