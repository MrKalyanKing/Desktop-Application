mod modules;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      
      if let Ok(conn) = modules::database::connection::establish_connection(app.handle()) {
        let _ = modules::database::migrations::run_migrations(&conn);
      }
      
      let _ = modules::tray::init_tray(app);

      if let Ok(settings_json) = modules::settings::storage::load_settings(app.handle()) {
        if settings_json.contains("\"startMinimized\":true") {
          if let Some(win) = app.get_webview_window("main") {
            let _ = win.hide();
          }
        }
      }

      if let Some(window) = app.get_webview_window("main") {
        modules::window::stealth::protect_from_capture(&window);
        let _ = modules::window::manager::init_window_position(&window);
      }
      
      Ok(())
    })
    .manage(modules::ai::commands::AiState::new())
    .on_window_event(|window, event| {
      if let tauri::WindowEvent::Moved(position) = event {
        let app_handle = window.app_handle();
        let mut s = modules::config::settings::load_settings(app_handle);
        s.x = position.x;
        s.y = position.y;
        let _ = modules::config::settings::save_settings(app_handle, &s);
      }
    })
    .invoke_handler(tauri::generate_handler![
      modules::window::commands::save_position,
      modules::ai::commands::ai_health_check,
      modules::ai::commands::ai_get_models,
      modules::ai::commands::ask_ai,
      modules::ai::commands::stream_ai,
      modules::ai::commands::cancel_ai,
      modules::screenshot::commands::capture_active_window_cmd,
      modules::screenshot::commands::capture_fullscreen_cmd,
      modules::screenshot::commands::capture_region_cmd,
      modules::screenshot::commands::start_scroll_capture_cmd,
      modules::screenshot::commands::open_region_selector_cmd,
      modules::ocr::commands::perform_ocr_cmd,
      modules::ai::commands::db_save_message_cmd,
      modules::ai::commands::db_load_history_cmd,
      modules::ai::commands::db_clear_history_cmd,
      modules::ai::commands::db_delete_message_cmd,
      modules::ai::commands::db_search_history_cmd,
      modules::ai::commands::db_export_markdown_cmd,
      modules::ai::commands::db_export_json_cmd,
      modules::settings::commands::load_settings_cmd,
      modules::settings::commands::save_settings_cmd,
      modules::settings::commands::set_startup_enabled_cmd,
      modules::settings::commands::is_startup_enabled_cmd,
      modules::window::stealth::set_stealth_mode,
      modules::window::stealth::verify_stealth
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
