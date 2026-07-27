use rusqlite::Connection;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageRecord {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub source: String,
}

fn get_db_path() -> PathBuf {
    PathBuf::from("C:\\KS-Vision\\database\\ks-vision-db")
}

pub fn init_db() -> Result<(), String> {
    let path = get_db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent database directories: {}", e))?;
    }
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS conversation_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            source TEXT NOT NULL
        )",
        [],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn save_message(role: &str, content: &str, source: &str) -> Result<(), String> {
    init_db()?;
    let path = get_db_path();
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    conn.execute(
        "INSERT INTO conversation_history (role, content, timestamp, source) VALUES (?1, ?2, ?3, ?4)",
        [role, content, &now.to_string(), source],
    ).map_err(|e| e.to_string())?;

    // Limit stored history to last 50 items to keep context bounds small
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM conversation_history",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    if count > 50 {
        let diff = count - 50;
        conn.execute(
            &format!("DELETE FROM conversation_history WHERE id IN (SELECT id FROM conversation_history ORDER BY id ASC LIMIT {})", diff),
            [],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn load_history(limit: u32) -> Result<Vec<MessageRecord>, String> {
    init_db()?;
    let path = get_db_path();
    let conn = Connection::open(path).map_err(|e| e.to_string())?;

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
    
    // Sort in ascending order (chronological) before returning
    history.reverse();
    Ok(history)
}

pub fn clear_history() -> Result<(), String> {
    init_db()?;
    let path = get_db_path();
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM conversation_history", []).map_err(|e| e.to_string())?;
    Ok(())
}
