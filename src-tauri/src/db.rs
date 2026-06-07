use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

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
        conn.busy_timeout(Duration::from_secs(5))?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.busy_timeout(Duration::from_secs(5))?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.lock_conn()?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS bookmarks (
                id          TEXT PRIMARY KEY,
                url         TEXT NOT NULL UNIQUE,
                title       TEXT NOT NULL,
                favicon     TEXT,
                created_at  INTEGER NOT NULL
            );
            DELETE FROM bookmarks
            WHERE rowid IN (
                SELECT rowid FROM (
                    SELECT rowid,
                           ROW_NUMBER() OVER (
                               PARTITION BY url
                               ORDER BY created_at DESC, rowid DESC
                           ) AS duplicate_rank
                    FROM bookmarks
                )
                WHERE duplicate_rank > 1
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_bookmarks_url ON bookmarks(url);
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
        ",
        )?;
        Ok(())
    }

    // ── Bookmarks ─────────────────────────────────────────────────────────────

    pub fn add_bookmark(&self, url: &str, title: &str) -> Result<Bookmark> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = unix_timestamp();
        let conn = self.lock_conn()?;
        conn.query_row(
            "INSERT INTO bookmarks (id, url, title, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(url) DO UPDATE SET title = excluded.title
             RETURNING id, url, title, favicon, created_at",
            params![id, url, title, now],
            bookmark_from_row,
        )
    }

    pub fn get_bookmarks(&self) -> Result<Vec<Bookmark>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, url, title, favicon, created_at FROM bookmarks ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], bookmark_from_row)?;
        rows.collect()
    }

    pub fn delete_bookmark(&self, id: &str) -> Result<()> {
        let conn = self.lock_conn()?;
        conn.execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn is_bookmarked(&self, url: &str) -> Result<bool> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM bookmarks WHERE url = ?1",
            params![url],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    // ── History ───────────────────────────────────────────────────────────────

    pub fn add_history(&self, url: &str, title: &str) -> Result<()> {
        let now = unix_timestamp();
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;
        let updated = tx.execute(
            "UPDATE history
             SET title = ?2,
                 visit_count = visit_count + 1,
                 last_visited = ?3
             WHERE url = ?1",
            params![url, title, now],
        )?;
        if updated == 0 {
            let inserted = tx.execute(
                "INSERT OR IGNORE INTO history (url, title, last_visited) VALUES (?1, ?2, ?3)",
                params![url, title, now],
            )?;
            if inserted == 0 {
                tx.execute(
                    "UPDATE history
                     SET title = ?2,
                         visit_count = visit_count + 1,
                         last_visited = ?3
                     WHERE url = ?1",
                    params![url, title, now],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_history(&self, limit: i64, offset: i64) -> Result<Vec<HistoryEntry>> {
        let limit = limit.clamp(1, 200);
        let offset = offset.max(0);
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, url, title, favicon, visit_count, last_visited
             FROM history ORDER BY last_visited DESC LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(params![limit, offset], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                favicon: row.get(3)?,
                visit_count: row.get(4)?,
                last_visited: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub fn search_history(&self, query: &str) -> Result<Vec<HistoryEntry>> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.lock_conn()?;
        let escaped_query = escape_like_pattern(query);
        let pattern = format!("%{}%", escaped_query);
        let mut stmt = conn.prepare(
            "SELECT id, url, title, favicon, visit_count, last_visited
             FROM history
             WHERE url LIKE ?1 ESCAPE '\\' COLLATE NOCASE OR title LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             ORDER BY last_visited DESC LIMIT 50",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                favicon: row.get(3)?,
                visit_count: row.get(4)?,
                last_visited: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub fn clear_history(&self) -> Result<()> {
        let conn = self.lock_conn()?;
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }

    // ── Settings ──────────────────────────────────────────────────────────────

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.lock_conn()?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    // ── Session ───────────────────────────────────────────────────────────────

    pub fn save_session(
        &self,
        tabs: &[crate::browser::TabInfo],
        active_id: Option<&str>,
    ) -> Result<()> {
        let tab_urls: Vec<&str> = tabs.iter().map(|t| t.url.as_str()).collect();
        let json = serde_json::to_string(&tab_urls)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let active_index = active_id
            .and_then(|id| tabs.iter().position(|tab| tab.id == id))
            .map(|idx| idx.to_string())
            .unwrap_or_default();
        self.set_setting("session_tabs", &json)?;
        self.set_setting("session_active_index", &active_index)?;
        Ok(())
    }

    pub fn load_session(&self) -> Result<(Vec<String>, Option<usize>)> {
        let tabs_json = self
            .get_setting("session_tabs")?
            .unwrap_or_else(|| "[]".to_string());
        let urls: Vec<String> = serde_json::from_str(&tabs_json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;
        let active = self
            .get_setting("session_active_index")?
            .and_then(|value| value.parse::<usize>().ok());
        Ok((urls, active))
    }

    fn lock_conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(
                "database lock poisoned",
            )))
        })
    }
}

fn escape_like_pattern(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' | '%' | '_' => escaped.push('\\'),
            _ => {}
        }
        escaped.push(ch);
    }
    escaped
}

fn bookmark_from_row(row: &rusqlite::Row<'_>) -> Result<Bookmark> {
    Ok(Bookmark {
        id: row.get(0)?,
        url: row.get(1)?,
        title: row.get(2)?,
        favicon: row.get(3)?,
        created_at: row.get(4)?,
    })
}

fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
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
    fn test_bookmark_add_updates_existing_url() {
        let db = test_db();
        let first = db.add_bookmark("https://github.com", "GitHub").unwrap();
        let second = db
            .add_bookmark("https://github.com", "GitHub Home")
            .unwrap();
        let all = db.get_bookmarks().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(first.id, second.id);
        assert_eq!(second.title, "GitHub Home");
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
        db.add_history("https://rust-lang.org", "Rust Programming Language")
            .unwrap();
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

    #[test]
    fn test_session_persists_active_index() {
        let db = test_db();
        let tabs = vec![
            crate::browser::TabInfo {
                id: "a".into(),
                url: "https://example.com".into(),
                title: "example.com".into(),
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
            crate::browser::TabInfo {
                id: "b".into(),
                url: "https://rust-lang.org".into(),
                title: "rust-lang.org".into(),
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
        ];

        db.save_session(&tabs, Some("b")).unwrap();

        let (urls, active_index) = db.load_session().unwrap();
        assert_eq!(
            urls,
            vec![
                "https://example.com".to_string(),
                "https://rust-lang.org".to_string()
            ]
        );
        assert_eq!(active_index, Some(1));
    }
}
