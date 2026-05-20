// ╔══════════════════════════════════════════════════════════════════════════╗
// ║   Web Storage – Cookies & LocalStorage für moderne Websites              ║
// ╚══════════════════════════════════════════════════════════════════════════╝

use std::collections::HashMap;
use std::path::PathBuf;
use rusqlite::{Connection, params, OptionalExtension};

pub struct WebStorage {
    db_path: PathBuf,
    conn: Option<Connection>,
    // In-Memory Cache für häufig verwendete Werte
    memory_cache: HashMap<String, String>,
}

impl WebStorage {
    /// WebStorage mit SQLite DB initialisieren
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        // Tabellen erstellen
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cookies (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                value TEXT NOT NULL,
                domain TEXT,
                path TEXT DEFAULT '/',
                expires TEXT,
                secure INTEGER DEFAULT 0,
                http_only INTEGER DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE TABLE IF NOT EXISTS local_storage (
                id INTEGER PRIMARY KEY,
                key TEXT UNIQUE NOT NULL,
                value TEXT NOT NULL,
                domain TEXT,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE INDEX IF NOT EXISTS idx_cookies_domain ON cookies(domain);
            CREATE INDEX IF NOT EXISTS idx_storage_domain ON local_storage(domain);"
        ).map_err(|e| format!("Failed to create tables: {}", e))?;

        Ok(WebStorage {
            db_path,
            conn: Some(conn),
            memory_cache: HashMap::new(),
        })
    }

    // ─── COOKIES ─────────────────────────────────────────────────────────────

    /// Cookie speichern (z.B. von Set-Cookie Header)
    pub fn set_cookie(&mut self, name: &str, value: &str, domain: Option<&str>, secure: bool) -> Result<(), String> {
        let conn = self.conn.as_ref().ok_or("Database not initialized")?;
        
        conn.execute(
            "INSERT OR REPLACE INTO cookies (name, value, domain, secure) VALUES (?1, ?2, ?3, ?4)",
            params![name, value, domain, if secure { 1 } else { 0 }],
        ).map_err(|e| format!("Failed to set cookie: {}", e))?;

        // Memory Cache aktualisieren
        self.memory_cache.insert(format!("cookie:{}", name), value.to_string());

        Ok(())
    }

    /// Cookie abrufen
    pub fn get_cookie(&self, name: &str) -> Result<Option<String>, String> {
        // Zuerst Memory Cache prüfen
        if let Some(val) = self.memory_cache.get(&format!("cookie:{}", name)) {
            return Ok(Some(val.clone()));
        }

        let conn = self.conn.as_ref().ok_or("Database not initialized")?;
        let mut stmt = conn.prepare("SELECT value FROM cookies WHERE name = ?1 LIMIT 1")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let value = stmt.query_row(params![name], |row| {
            row.get::<_, String>(0)
        }).optional()
            .map_err(|e| format!("Failed to query cookie: {}", e))?;

        Ok(value)
    }

    /// Alle Cookies für Domain abrufen
    pub fn get_cookies_for_domain(&self, domain: &str) -> Result<Vec<(String, String)>, String> {
        let conn = self.conn.as_ref().ok_or("Database not initialized")?;
        let mut stmt = conn.prepare("SELECT name, value FROM cookies WHERE domain = ?1 OR domain IS NULL")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let cookies = stmt.query_map(params![domain], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| format!("Failed to query cookies: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect cookies: {}", e))?;

        Ok(cookies)
    }

    /// Cookie löschen
    pub fn delete_cookie(&mut self, name: &str) -> Result<(), String> {
        let conn = self.conn.as_ref().ok_or("Database not initialized")?;
        conn.execute("DELETE FROM cookies WHERE name = ?1", params![name])
            .map_err(|e| format!("Failed to delete cookie: {}", e))?;

        self.memory_cache.remove(&format!("cookie:{}", name));
        Ok(())
    }

    // ─── LOCALSTORAGE ─────────────────────────────────────────────────────────

    /// LocalStorage Item speichern (für Website-eigene Daten)
    pub fn set_local_storage(&mut self, key: &str, value: &str, domain: Option<&str>) -> Result<(), String> {
        let conn = self.conn.as_ref().ok_or("Database not initialized")?;
        
        conn.execute(
            "INSERT OR REPLACE INTO local_storage (key, value, domain) VALUES (?1, ?2, ?3)",
            params![key, value, domain],
        ).map_err(|e| format!("Failed to set local storage: {}", e))?;

        // Memory Cache aktualisieren
        self.memory_cache.insert(format!("storage:{}", key), value.to_string());

        Ok(())
    }

    /// LocalStorage Item abrufen
    pub fn get_local_storage(&self, key: &str) -> Result<Option<String>, String> {
        // Zuerst Memory Cache prüfen
        if let Some(val) = self.memory_cache.get(&format!("storage:{}", key)) {
            return Ok(Some(val.clone()));
        }

        let conn = self.conn.as_ref().ok_or("Database not initialized")?;
        let mut stmt = conn.prepare("SELECT value FROM local_storage WHERE key = ?1 LIMIT 1")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let value = stmt.query_row(params![key], |row| {
            row.get::<_, String>(0)
        }).optional()
            .map_err(|e| format!("Failed to query local storage: {}", e))?;

        Ok(value)
    }

    /// LocalStorage Item löschen
    pub fn delete_local_storage(&mut self, key: &str) -> Result<(), String> {
        let conn = self.conn.as_ref().ok_or("Database not initialized")?;
        conn.execute("DELETE FROM local_storage WHERE key = ?1", params![key])
            .map_err(|e| format!("Failed to delete local storage: {}", e))?;

        self.memory_cache.remove(&format!("storage:{}", key));
        Ok(())
    }

    /// LocalStorage räumen (z.B. beim Löschen von Browserdaten)
    pub fn clear_local_storage(&mut self) -> Result<(), String> {
        let conn = self.conn.as_ref().ok_or("Database not initialized")?;
        conn.execute("DELETE FROM local_storage", [])
            .map_err(|e| format!("Failed to clear local storage: {}", e))?;

        self.memory_cache.retain(|k, _| !k.starts_with("storage:"));
        Ok(())
    }

    /// Memory Cache räumen
    pub fn clear_memory_cache(&mut self) {
        self.memory_cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_cookies() {
        let db_path = PathBuf::from("/tmp/test_cookies.db");
        let _ = fs::remove_file(&db_path);

        let mut storage = WebStorage::new(db_path.clone()).unwrap();
        
        storage.set_cookie("session_id", "abc123", Some("youtube.com"), false).unwrap();
        assert_eq!(storage.get_cookie("session_id").unwrap(), Some("abc123".to_string()));
        
        storage.delete_cookie("session_id").unwrap();
        assert_eq!(storage.get_cookie("session_id").unwrap(), None);

        let _ = fs::remove_file(&db_path);
    }

    #[test]
    fn test_local_storage() {
        let db_path = PathBuf::from("/tmp/test_storage.db");
        let _ = fs::remove_file(&db_path);

        let mut storage = WebStorage::new(db_path.clone()).unwrap();
        
        storage.set_local_storage("user_prefs", r#"{"theme":"dark"}"#, Some("wikipedia.org")).unwrap();
        assert_eq!(
            storage.get_local_storage("user_prefs").unwrap(), 
            Some(r#"{"theme":"dark"}"#.to_string())
        );
        
        let _ = fs::remove_file(&db_path);
    }
}
