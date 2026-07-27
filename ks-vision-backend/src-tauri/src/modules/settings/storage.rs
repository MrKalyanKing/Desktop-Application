use crate::modules::database::connection::establish_connection;
use crate::modules::settings::defaults::get_default_settings;
use tauri::AppHandle;
use std::process::Command;

pub fn load_settings(app: &AppHandle) -> Result<String, String> {
    let conn = establish_connection(app)?;
    let _ = crate::modules::database::migrations::run_migrations(&conn);

    let mut stmt = conn.prepare("SELECT value FROM application_settings WHERE key = 'app_preferences'")
        .map_err(|e| e.to_string())?;
    
    let res = stmt.query_row([], |row| row.get::<_, String>(0));
    
    match res {
        Ok(val) => Ok(val),
        Err(_) => {
            let default_str = get_default_settings();
            let _ = conn.execute(
                "INSERT OR REPLACE INTO application_settings (key, value) VALUES ('app_preferences', ?1)",
                [default_str],
            );
            Ok(default_str.to_string())
        }
    }
}

pub fn save_settings(app: &AppHandle, settings_json: &str) -> Result<(), String> {
    let conn = establish_connection(app)?;
    let _ = crate::modules::database::migrations::run_migrations(&conn);
    
    conn.execute(
        "INSERT OR REPLACE INTO application_settings (key, value) VALUES ('app_preferences', ?1)",
        [settings_json],
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

pub fn set_startup_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;
        let path_str = current_exe.to_str()
            .ok_or("Executable path is not valid UTF-8")?;

        let status = if enabled {
            Command::new("powershell")
                .arg("-Command")
                .arg(format!(
                    "Set-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run' -Name 'KSVision' -Value '\"{}\"'",
                    path_str
                ))
                .status()
        } else {
            Command::new("powershell")
                .arg("-Command")
                .arg("Remove-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run' -Name 'KSVision' -ErrorAction SilentlyContinue")
                .status()
        };

        match status {
            Ok(s) if s.success() => {}
            _ => return Err("Failed to update Windows Registry startup state".to_string()),
        }
    }
    Ok(())
}

pub fn is_startup_enabled() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .arg("-Command")
            .arg("Get-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run' -Name 'KSVision' -ErrorAction SilentlyContinue")
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                Ok(stdout.contains("KSVision"))
            }
            _ => Ok(false),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(false)
    }
}
