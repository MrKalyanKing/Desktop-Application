pub mod modules;

use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  std::panic::set_hook(Box::new(|info| {
      let backtrace = std::backtrace::Backtrace::capture();
      let panic_msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
          *s
      } else if let Some(s) = info.payload().downcast_ref::<String>() {
          s.as_str()
      } else {
          "unknown panic"
      };
      let location = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_else(|| "unknown location".to_string());
      let log_content = format!("Panic occurred: {}\nLocation: {}\nBacktrace:\n{:?}", panic_msg, location, backtrace);
      let _ = std::fs::write("panic_log.txt", log_content);
  }));

  tauri::Builder::default()
    // Must be first: a second launch (Start Menu key while already running) just shows
    // the existing overlay instead of starting a copy that can't register any keys.
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
      let _ = modules::shortcuts::window::show_overlay(app, true);
    }))
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .plugin(tauri_plugin_clipboard_manager::init())
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

      if let Some(window) = app.get_webview_window("main") {
        // Ghost mode: start hidden, no taskbar entry
        let _ = modules::window::ghost::enter_ghost_mode(&window);
        modules::window::stealth::protect_from_capture(&window);
        let _ = modules::window::manager::init_window_position(&window);
        // Hide OS cursor over the main window (screen-share friendly)
        let _ = window.set_cursor_visible(false);
      }

      // All global keys live in one Rust registry, registered once here; see modules::shortcuts.
      modules::shortcuts::init(app.handle());
      
      Ok(())
    })
    .manage(modules::ai::commands::AiState::new())
    .manage(modules::audio::commands::AudioState::new())
    .manage(modules::shortcuts::ShortcutState::new())
    .manage(modules::shortcuts::ops::OpRegistry::new())
    .on_window_event(|window, event| {
      match event {
        WindowEvent::Moved(position) => {
          let app_handle = window.app_handle();
          let mut s = modules::config::settings::load_settings(app_handle);
          s.x = position.x;
          s.y = position.y;
          let _ = modules::config::settings::save_settings(app_handle, &s);
        }
        WindowEvent::CloseRequested { api, .. } => {
          // X / close on main → ghost mode instead of quitting (quit via tray)
          if window.label() == "main" {
            api.prevent_close();
            let _ = modules::shortcuts::window::hide_overlay(window.app_handle());
          }
        }
        _ => {}
      }
    })
    .invoke_handler(tauri::generate_handler![
      modules::window::commands::save_position,
      modules::window::commands::set_os_cursor_visible,
      modules::window::ghost::show_window,
      modules::window::ghost::hide_window,
      modules::window::ghost::toggle_ghost_mode_cmd,
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
      modules::screenshot::commands::analyze_screenshot_cmd,
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
      modules::settings::commands::test_gemini_connection,
      modules::window::stealth::set_stealth_mode,
      modules::window::stealth::verify_stealth,
      modules::audio::commands::start_audio_capture,
      modules::audio::commands::stop_audio_capture,
      modules::audio::commands::get_audio_capture_state,
      modules::audio::commands::register_ai_audio_output,
      modules::audio::commands::set_ai_generating_state,
      modules::audio::commands::pop_next_pending_question,
      modules::cloud_sync::commands::cloud_configure,
      modules::cloud_sync::commands::cloud_get_status,
      modules::cloud_sync::commands::cloud_login,
      modules::cloud_sync::commands::cloud_logout,
      modules::cloud_sync::commands::cloud_set_session,
      modules::cloud_sync::commands::cloud_flush_queue,
      modules::shortcuts::get_shortcut_status_cmd,
      modules::shortcuts::set_last_answer_cmd,
      modules::shortcuts::show_overlay_cmd,
      modules::shortcuts::hide_overlay_cmd,
      modules::shortcuts::ops::begin_op_cmd,
      modules::shortcuts::ops::end_op_cmd,
      modules::shortcuts::window::release_focus_cmd
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
