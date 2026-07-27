use rusqlite::Connection;

pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    // 1. Create conversation history table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS conversation_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            source TEXT NOT NULL
        )",
        [],
    ).map_err(|e| format!("Migration failed (history): {}", e))?;

    // 2. Create settings table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS application_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    ).map_err(|e| format!("Migration failed (settings): {}", e))?;

    Ok(())
}
