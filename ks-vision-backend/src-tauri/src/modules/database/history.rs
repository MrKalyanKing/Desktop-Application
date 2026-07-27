use crate::modules::database::connection::establish_connection;
use serde::{Serialize, Deserialize};
use rusqlite::params;
use tauri::AppHandle;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageRecord {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub source: String,
}

pub fn save_message(app: &AppHandle, role: &str, content: &str, source: &str) -> Result<(), String> {
    let conn = establish_connection(app)?;
    let _ = crate::modules::database::migrations::run_migrations(&conn);
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
        
    conn.execute(
        "INSERT INTO conversation_history (role, content, timestamp, source) VALUES (?1, ?2, ?3, ?4)",
        params![role, content, now, source],
    ).map_err(|e| e.to_string())?;

    // Prune history to limit size (last 100 entries)
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM conversation_history",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    if count > 100 {
        let diff = count - 100;
        conn.execute(
            &format!("DELETE FROM conversation_history WHERE id IN (SELECT id FROM conversation_history ORDER BY id ASC LIMIT {})", diff),
            [],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn load_history(app: &AppHandle, limit: u32) -> Result<Vec<MessageRecord>, String> {
    let conn = establish_connection(app)?;
    let _ = crate::modules::database::migrations::run_migrations(&conn);
    
    let mut stmt = conn.prepare(
        "SELECT id, role, content, timestamp, source FROM conversation_history ORDER BY id DESC LIMIT ?1"
    ).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([limit], |row| {
        Ok(MessageRecord {
            id: row.get(0)?,
            role: row.get(1)?,
            content: row.get(2)?,
            timestamp: row.get(3)?,
            source: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut history = Vec::new();
    for row in rows {
        if let Ok(rec) = row {
            history.push(rec);
        }
    }
    history.reverse();
    Ok(history)
}

pub fn clear_history(app: &AppHandle) -> Result<(), String> {
    let conn = establish_connection(app)?;
    let _ = crate::modules::database::migrations::run_migrations(&conn);
    conn.execute("DELETE FROM conversation_history", []).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_message(app: &AppHandle, id: i64) -> Result<(), String> {
    let conn = establish_connection(app)?;
    let _ = crate::modules::database::migrations::run_migrations(&conn);
    conn.execute("DELETE FROM conversation_history WHERE id = ?1", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn search_history(app: &AppHandle, query: &str) -> Result<Vec<MessageRecord>, String> {
    let conn = establish_connection(app)?;
    let _ = crate::modules::database::migrations::run_migrations(&conn);
    let formatted_query = format!("%{}%", query);
    
    let mut stmt = conn.prepare(
        "SELECT id, role, content, timestamp, source FROM conversation_history WHERE content LIKE ?1 ORDER BY id ASC"
    ).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([formatted_query], |row| {
        Ok(MessageRecord {
            id: row.get(0)?,
            role: row.get(1)?,
            content: row.get(2)?,
            timestamp: row.get(3)?,
            source: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut history = Vec::new();
    for row in rows {
        if let Ok(rec) = row {
            history.push(rec);
        }
    }
    Ok(history)
}

pub fn export_markdown(app: &AppHandle) -> Result<String, String> {
    let history = load_history(app, 1000)?;
    let mut md = String::new();
    md.push_str("# KS Vision Conversation Export\n\n");
    for item in history {
        let role_label = if item.role == "user" { "**User**" } else { "**AI Copilot**" };
        let source_label = if item.source.is_empty() { String::new() } else { format!(" via {}", item.source) };
        md.push_str(&format!("### {}{}\n\n{}\n\n---\n\n", role_label, source_label, item.content));
    }
    Ok(md)
}

pub fn export_json(app: &AppHandle) -> Result<String, String> {
    let history = load_history(app, 1000)?;
    serde_json::to_string_pretty(&history).map_err(|e| e.to_string())
}
