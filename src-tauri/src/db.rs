use rusqlite::{Connection, Result, params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

pub struct Db {
    pub conn: Mutex<Connection>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Bookmark {
    pub id: String,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub visit_count: i64,
    pub last_visited: i64,
}

impl Db {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn: Mutex::new(conn) };
        db.init_schema()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn: Mutex::new(conn) };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS bookmarks (
                id          TEXT PRIMARY KEY,
                url         TEXT NOT NULL,
                title       TEXT NOT NULL,
                favicon     TEXT,
                created_at  INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS history (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                url          TEXT NOT NULL UNIQUE,
                title        TEXT NOT NULL,
                favicon      TEXT,
                visit_count  INTEGER DEFAULT 1,
                last_visited INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_history_last_visited ON history(last_visited DESC);
            CREATE INDEX IF NOT EXISTS idx_history_url ON history(url);
            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
        ")?;
        Ok(())
    }

    // ── Bookmarks ─────────────────────────────────────────────────────────────

    pub fn add_bookmark(&self, url: &str, title: &str) -> Result<Bookmark> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO bookmarks (id, url, title, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, url, title, now],
        )?;
        Ok(Bookmark { id, url: url.to_string(), title: title.to_string(), favicon: None, created_at: now })
    }

    pub fn get_bookmarks(&self) -> Result<Vec<Bookmark>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, title, favicon, created_at FROM bookmarks ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| Ok(Bookmark {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            favicon: row.get(3)?,
            created_at: row.get(4)?,
        }))?;
        rows.collect()
    }

    pub fn delete_bookmark(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn is_bookmarked(&self, url: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM bookmarks WHERE url = ?1",
            params![url],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    // ── History ───────────────────────────────────────────────────────────────

    pub fn add_history(&self, url: &str, title: &str) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO history (url, title, last_visited) VALUES (?1, ?2, ?3)
             ON CONFLICT(url) DO UPDATE SET
                title = excluded.title,
                visit_count = visit_count + 1,
                last_visited = excluded.last_visited",
            params![url, title, now],
        )?;
        Ok(())
    }

    pub fn get_history(&self, limit: i64, offset: i64) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, title, favicon, visit_count, last_visited
             FROM history ORDER BY last_visited DESC LIMIT ?1 OFFSET ?2"
        )?;
        let rows = stmt.query_map(params![limit, offset], |row| Ok(HistoryEntry {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            favicon: row.get(3)?,
            visit_count: row.get(4)?,
            last_visited: row.get(5)?,
        }))?;
        rows.collect()
    }

    pub fn search_history(&self, query: &str) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = conn.prepare(
            "SELECT id, url, title, favicon, visit_count, last_visited
             FROM history
             WHERE lower(url) LIKE ?1 OR lower(title) LIKE ?1
             ORDER BY last_visited DESC LIMIT 50"
        )?;
        let rows = stmt.query_map(params![pattern], |row| Ok(HistoryEntry {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            favicon: row.get(3)?,
            visit_count: row.get(4)?,
            last_visited: row.get(5)?,
        }))?;
        rows.collect()
    }

    pub fn clear_history(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }

    // ── Settings ──────────────────────────────────────────────────────────────

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        ).optional()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Db {
        Db::open_in_memory().unwrap()
    }

    #[test]
    fn test_bookmark_add_and_get() {
        let db = test_db();
        let bm = db.add_bookmark("https://github.com", "GitHub").unwrap();
        assert_eq!(bm.url, "https://github.com");
        assert_eq!(bm.title, "GitHub");
        let all = db.get_bookmarks().unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_bookmark_is_bookmarked_and_delete() {
        let db = test_db();
        let bm = db.add_bookmark("https://rust-lang.org", "Rust").unwrap();
        assert!(db.is_bookmarked("https://rust-lang.org").unwrap());
        db.delete_bookmark(&bm.id).unwrap();
        assert!(!db.is_bookmarked("https://rust-lang.org").unwrap());
    }

    #[test]
    fn test_history_upsert_increments_visit_count() {
        let db = test_db();
        db.add_history("https://rust-lang.org", "Rust").unwrap();
        db.add_history("https://rust-lang.org", "Rust").unwrap();
        db.add_history("https://rust-lang.org", "Rust").unwrap();
        let h = db.get_history(10, 0).unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].visit_count, 3);
    }

    #[test]
    fn test_history_search_filters_by_url_and_title() {
        let db = test_db();
        db.add_history("https://rust-lang.org", "Rust Programming Language").unwrap();
        db.add_history("https://github.com", "GitHub").unwrap();
        // Search by URL
        let r1 = db.search_history("rust-lang").unwrap();
        assert_eq!(r1.len(), 1);
        assert_eq!(r1[0].url, "https://rust-lang.org");
        // Search by title
        let r2 = db.search_history("programming").unwrap();
        assert_eq!(r2.len(), 1);
        // No match
        let r3 = db.search_history("zzznomatch").unwrap();
        assert_eq!(r3.len(), 0);
    }

    #[test]
    fn test_settings_get_set() {
        let db = test_db();
        assert!(db.get_setting("theme").unwrap().is_none());
        db.set_setting("theme", "dark").unwrap();
        assert_eq!(db.get_setting("theme").unwrap(), Some("dark".into()));
        // Overwrite
        db.set_setting("theme", "light").unwrap();
        assert_eq!(db.get_setting("theme").unwrap(), Some("light".into()));
    }
}
