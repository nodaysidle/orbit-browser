use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

pub struct Db {
    pub conn: Mutex<Connection>,
}

const CURRENT_SCHEMA_VERSION: i64 = 2;
const MAX_HISTORY_ENTRIES: i64 = 10_000;

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProjectTab {
    pub id: String,
    pub url: String,
    pub title: String,
    pub domain: Option<String>,
    pub position: i64,
    pub is_active: bool,
    pub added_at: i64,
    pub last_seen_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub source: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_opened_at: Option<i64>,
    pub last_closed_at: Option<i64>,
    pub tabs: Vec<ProjectTab>,
    pub domains: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProjectSuggestion {
    pub name: String,
    pub slug: String,
    pub reason: String,
    pub confidence: i64,
    pub domains: Vec<String>,
}

impl Db {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        db.trim_history()?;
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
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_version (
                version     INTEGER PRIMARY KEY,
                applied_at  INTEGER NOT NULL
            );
        ",
        )?;

        let current_version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get::<_, Option<i64>>(0)
            })?
            .unwrap_or(0);

        if current_version < 1 {
            Self::apply_schema_v1(&conn)?;
        }
        if current_version < 2 {
            Self::apply_schema_v2(&conn)?;
        }

        if current_version < CURRENT_SCHEMA_VERSION {
            conn.execute(
                "INSERT OR REPLACE INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                params![CURRENT_SCHEMA_VERSION, unix_timestamp()],
            )?;
        }

        Ok(())
    }

    fn apply_schema_v1(conn: &Connection) -> Result<()> {
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

    fn apply_schema_v2(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS projects (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                slug            TEXT NOT NULL UNIQUE,
                source          TEXT NOT NULL DEFAULT 'manual',
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL,
                last_opened_at  INTEGER,
                last_closed_at  INTEGER,
                archived_at     INTEGER
            );
            CREATE TABLE IF NOT EXISTS project_tabs (
                id              TEXT PRIMARY KEY,
                project_id      TEXT NOT NULL,
                url             TEXT NOT NULL,
                title           TEXT NOT NULL,
                domain          TEXT,
                position        INTEGER NOT NULL,
                is_active       INTEGER NOT NULL DEFAULT 0,
                added_at        INTEGER NOT NULL,
                last_seen_at    INTEGER NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS project_domains (
                project_id      TEXT NOT NULL,
                domain          TEXT NOT NULL,
                weight          INTEGER NOT NULL DEFAULT 1,
                first_seen_at   INTEGER NOT NULL,
                last_seen_at    INTEGER NOT NULL,
                PRIMARY KEY(project_id, domain),
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS project_events (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id      TEXT NOT NULL,
                event_type      TEXT NOT NULL,
                url             TEXT,
                title           TEXT,
                domain          TEXT,
                created_at      INTEGER NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_projects_last_opened ON projects(last_opened_at DESC);
            CREATE INDEX IF NOT EXISTS idx_project_tabs_project_position ON project_tabs(project_id, position);
            CREATE INDEX IF NOT EXISTS idx_project_tabs_domain ON project_tabs(domain);
            CREATE INDEX IF NOT EXISTS idx_project_domains_domain ON project_domains(domain);
            CREATE INDEX IF NOT EXISTS idx_project_events_project_created ON project_events(project_id, created_at DESC);
        ",
        )?;
        Self::migrate_session_to_default_project(conn)?;
        Ok(())
    }

    fn migrate_session_to_default_project(conn: &Connection) -> Result<()> {
        let existing: i64 =
            conn.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))?;
        if existing > 0 {
            return Ok(());
        }
        let tabs_json: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'session_tabs'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let Some(tabs_json) = tabs_json else {
            return Ok(());
        };
        let urls: Vec<String> = serde_json::from_str(&tabs_json).unwrap_or_default();
        let urls: Vec<String> = urls.into_iter().filter(|url| is_web_url(url)).collect();
        if urls.is_empty() {
            return Ok(());
        }
        let active_index = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'session_active_index'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let now = unix_timestamp();
        let project_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (id, name, slug, source, created_at, updated_at, last_opened_at, last_closed_at)
             VALUES (?1, 'Current Work', 'current-work', 'migration', ?2, ?2, ?2, ?2)",
            params![project_id, now],
        )?;
        insert_project_tabs(conn, &project_id, &urls, active_index, now)?;
        insert_project_domains(conn, &project_id, &urls, now)?;
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

    // ── Projects ──────────────────────────────────────────────────────────────

    pub fn create_project_from_tabs(
        &self,
        name: &str,
        tabs: &[crate::browser::TabInfo],
        active_id: Option<&str>,
        source: &str,
    ) -> Result<Project> {
        let web_tabs = web_tabs(tabs);
        let now = unix_timestamp();
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;
        let slug = unique_project_slug(&tx, &slugify(name))?;
        let id = uuid::Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO projects (id, name, slug, source, created_at, updated_at, last_opened_at, last_closed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?5, ?5)",
            params![id, clean_project_name(name), slug, source, now],
        )?;
        persist_project_snapshot(&tx, &id, &web_tabs, active_id, now)?;
        let project = get_project_from_conn(&tx, &id)?;
        tx.commit()?;
        Ok(project)
    }

    pub fn update_project_from_tabs(
        &self,
        project_id: &str,
        tabs: &[crate::browser::TabInfo],
        active_id: Option<&str>,
    ) -> Result<Project> {
        let web_tabs = web_tabs(tabs);
        let now = unix_timestamp();
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE projects SET updated_at = ?2, last_closed_at = ?2 WHERE id = ?1 AND archived_at IS NULL",
            params![project_id, now],
        )?;
        persist_project_snapshot(&tx, project_id, &web_tabs, active_id, now)?;
        let project = get_project_from_conn(&tx, project_id)?;
        tx.commit()?;
        Ok(project)
    }

    pub fn save_detected_project(
        &self,
        tabs: &[crate::browser::TabInfo],
        active_id: Option<&str>,
    ) -> Result<Option<Project>> {
        let web_tabs = web_tabs(tabs);
        let Some(suggestion) = detect_project_from_tabs(&web_tabs) else {
            return Ok(None);
        };
        let now = unix_timestamp();
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;
        let existing_id: Option<String> = tx
            .query_row(
                "SELECT id FROM projects WHERE slug = ?1 AND archived_at IS NULL",
                params![suggestion.slug],
                |row| row.get(0),
            )
            .optional()?;
        let project_id = if let Some(id) = existing_id {
            tx.execute(
                "UPDATE projects SET name = ?2, source = 'detected', updated_at = ?3, last_closed_at = ?3 WHERE id = ?1",
                params![id, suggestion.name, now],
            )?;
            id
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO projects (id, name, slug, source, created_at, updated_at, last_opened_at, last_closed_at)
                 VALUES (?1, ?2, ?3, 'detected', ?4, ?4, ?4, ?4)",
                params![id, suggestion.name, suggestion.slug, now],
            )?;
            id
        };
        persist_project_snapshot(&tx, &project_id, &web_tabs, active_id, now)?;
        let project = get_project_from_conn(&tx, &project_id)?;
        tx.commit()?;
        Ok(Some(project))
    }

    pub fn get_projects(&self) -> Result<Vec<Project>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id FROM projects WHERE archived_at IS NULL ORDER BY COALESCE(last_opened_at, updated_at, created_at) DESC, updated_at DESC",
        )?;
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>>>()?;
        ids.into_iter()
            .map(|id| get_project_from_conn(&conn, &id))
            .collect()
    }

    pub fn open_project(&self, project_id: &str) -> Result<Project> {
        let now = unix_timestamp();
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE projects SET last_opened_at = ?2, updated_at = ?2 WHERE id = ?1 AND archived_at IS NULL",
            params![project_id, now],
        )?;
        conn.execute(
            "INSERT INTO project_events (project_id, event_type, created_at) VALUES (?1, 'opened', ?2)",
            params![project_id, now],
        )?;
        get_project_from_conn(&conn, project_id)
    }

    pub fn delete_project(&self, project_id: &str) -> Result<()> {
        let now = unix_timestamp();
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE projects SET archived_at = ?2, updated_at = ?2 WHERE id = ?1",
            params![project_id, now],
        )?;
        Ok(())
    }

    pub fn detect_project_suggestion(
        &self,
        tabs: &[crate::browser::TabInfo],
    ) -> Option<ProjectSuggestion> {
        detect_project_from_tabs(&web_tabs(tabs))
    }

    pub fn save_active_project_snapshot_from_settings(
        &self,
        tabs: &[crate::browser::TabInfo],
        active_id: Option<&str>,
    ) -> Result<Option<Project>> {
        if self.project_restore_guard_active()? {
            return Ok(None);
        }
        let Some(project_id) = self.get_setting("active_project_id")? else {
            return Ok(None);
        };
        let project_id = project_id.trim();
        if project_id.is_empty() {
            return Ok(None);
        }
        self.update_project_from_tabs(project_id, tabs, active_id)
            .map(Some)
    }

    fn project_restore_guard_active(&self) -> Result<bool> {
        let Some(started) = self.get_setting("project_restore_started_at")? else {
            return Ok(false);
        };
        let Ok(started_at) = started.trim().parse::<i64>() else {
            return Ok(false);
        };
        Ok(unix_timestamp().saturating_sub(started_at) <= 30)
    }

    pub fn trim_history(&self) -> Result<()> {
        let conn = self.lock_conn()?;
        conn.execute(
            "DELETE FROM history
             WHERE id NOT IN (
                 SELECT id FROM history ORDER BY last_visited DESC, id DESC LIMIT ?1
             )",
            params![MAX_HISTORY_ENTRIES],
        )?;
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
        match self.conn.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                eprintln!("orbit: database lock poisoned; recovering inner connection");
                Ok(poisoned.into_inner())
            }
        }
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

fn get_project_from_conn(conn: &Connection, project_id: &str) -> Result<Project> {
    let (id, name, slug, source, created_at, updated_at, last_opened_at, last_closed_at) = conn
        .query_row(
            "SELECT id, name, slug, source, created_at, updated_at, last_opened_at, last_closed_at
         FROM projects WHERE id = ?1 AND archived_at IS NULL",
            params![project_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                ))
            },
        )?;
    let mut tabs_stmt = conn.prepare(
        "SELECT id, url, title, domain, position, is_active, added_at, last_seen_at
         FROM project_tabs WHERE project_id = ?1 ORDER BY position ASC",
    )?;
    let tabs = tabs_stmt
        .query_map(params![project_id], |row| {
            Ok(ProjectTab {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                domain: row.get(3)?,
                position: row.get(4)?,
                is_active: row.get::<_, i64>(5)? == 1,
                added_at: row.get(6)?,
                last_seen_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    let mut domains_stmt = conn.prepare(
        "SELECT domain FROM project_domains WHERE project_id = ?1 ORDER BY weight DESC, last_seen_at DESC, domain ASC",
    )?;
    let domains = domains_stmt
        .query_map(params![project_id], |row| row.get(0))?
        .collect::<Result<Vec<String>>>()?;
    Ok(Project {
        id,
        name,
        slug,
        source,
        created_at,
        updated_at,
        last_opened_at,
        last_closed_at,
        tabs,
        domains,
    })
}

fn persist_project_snapshot(
    conn: &Connection,
    project_id: &str,
    tabs: &[crate::browser::TabInfo],
    active_id: Option<&str>,
    now: i64,
) -> Result<()> {
    conn.execute(
        "DELETE FROM project_tabs WHERE project_id = ?1",
        params![project_id],
    )?;
    conn.execute(
        "DELETE FROM project_domains WHERE project_id = ?1",
        params![project_id],
    )?;
    for (idx, tab) in tabs.iter().enumerate() {
        conn.execute(
            "INSERT INTO project_tabs (id, project_id, url, title, domain, position, is_active, added_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                uuid::Uuid::new_v4().to_string(),
                project_id,
                tab.url,
                useful_title(tab),
                url_domain(&tab.url),
                idx as i64,
                if active_id.is_some_and(|id| id == tab.id) { 1 } else { 0 },
                now,
            ],
        )?;
    }
    insert_project_domains(
        conn,
        project_id,
        &tabs.iter().map(|tab| tab.url.clone()).collect::<Vec<_>>(),
        now,
    )?;
    conn.execute(
        "INSERT INTO project_events (project_id, event_type, created_at) VALUES (?1, 'snapshot_saved', ?2)",
        params![project_id, now],
    )?;
    Ok(())
}

fn insert_project_tabs(
    conn: &Connection,
    project_id: &str,
    urls: &[String],
    active_index: usize,
    now: i64,
) -> Result<()> {
    let tabs: Vec<crate::browser::TabInfo> = urls
        .iter()
        .enumerate()
        .map(|(idx, url)| crate::browser::TabInfo {
            id: format!("migrated-{idx}"),
            url: url.clone(),
            title: crate::browser::title_from_url(url),
            loading: false,
            can_go_back: false,
            can_go_forward: false,
        })
        .collect();
    let active_id = tabs.get(active_index).map(|tab| tab.id.as_str());
    persist_project_snapshot(conn, project_id, &tabs, active_id, now)
}

fn insert_project_domains(
    conn: &Connection,
    project_id: &str,
    urls: &[String],
    now: i64,
) -> Result<()> {
    let mut counts: BTreeMap<String, i64> = BTreeMap::new();
    for url in urls {
        if let Some(domain) = url_domain(url) {
            *counts.entry(domain).or_insert(0) += 1;
        }
    }
    for (domain, weight) in counts {
        conn.execute(
            "INSERT OR REPLACE INTO project_domains (project_id, domain, weight, first_seen_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![project_id, domain, weight, now],
        )?;
    }
    Ok(())
}

fn web_tabs(tabs: &[crate::browser::TabInfo]) -> Vec<crate::browser::TabInfo> {
    tabs.iter()
        .filter(|tab| is_web_url(&tab.url))
        .cloned()
        .collect()
}

fn is_web_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn useful_title(tab: &crate::browser::TabInfo) -> String {
    let title = tab.title.trim();
    if !title.is_empty() && !title.eq_ignore_ascii_case("new tab") {
        return title.to_string();
    }
    crate::browser::title_from_url(&tab.url)
}

fn url_domain(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed
        .host_str()?
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

fn github_repo_name(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.trim_start_matches("www.");
    if host != "github.com" {
        return None;
    }
    let mut parts = parsed.path_segments()?;
    let _owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    if repo.is_empty() {
        None
    } else {
        Some(repo.trim_end_matches(".git").to_string())
    }
}

fn detect_project_from_tabs(tabs: &[crate::browser::TabInfo]) -> Option<ProjectSuggestion> {
    if tabs.len() < 2 {
        return None;
    }
    let domains: BTreeSet<String> = tabs.iter().filter_map(|tab| url_domain(&tab.url)).collect();
    let has_localhost = domains
        .iter()
        .any(|domain| domain == "localhost" || domain == "127.0.0.1" || domain.ends_with(".local"));
    let has_docs = domains.iter().any(|domain| is_docs_domain(domain));
    let repo = tabs.iter().find_map(|tab| github_repo_name(&tab.url));
    if let Some(repo) = repo {
        if has_localhost || has_docs {
            let name = titleize_slug(&repo);
            return Some(ProjectSuggestion {
                slug: slugify(&name),
                name,
                reason: "github_repo_with_builder_context".to_string(),
                confidence: 95,
                domains: domains.into_iter().collect(),
            });
        }
    }
    let mut counts: BTreeMap<String, i64> = BTreeMap::new();
    for tab in tabs {
        if let Some(domain) = url_domain(&tab.url) {
            *counts.entry(domain).or_insert(0) += 1;
        }
    }
    let (domain, count) = counts.into_iter().max_by_key(|(_, count)| *count)?;
    if count >= 3 && (is_docs_domain(&domain) || domain.contains("github") || domain == "localhost")
    {
        let name = titleize_slug(&domain.replace('.', "-"));
        return Some(ProjectSuggestion {
            slug: slugify(&name),
            name,
            reason: "repeated_builder_domain".to_string(),
            confidence: 80,
            domains: vec![domain],
        });
    }
    None
}

fn is_docs_domain(domain: &str) -> bool {
    matches!(
        domain,
        "docs.rs"
            | "tauri.app"
            | "developer.mozilla.org"
            | "npmjs.com"
            | "www.npmjs.com"
            | "stackoverflow.com"
    ) || domain.starts_with("docs.")
}

fn clean_project_name(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        "Current Work".to_string()
    } else {
        value.chars().take(80).collect()
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "project".to_string()
    } else {
        slug
    }
}

fn titleize_slug(value: &str) -> String {
    value
        .split(['-', '_', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn unique_project_slug(conn: &Connection, base_slug: &str) -> Result<String> {
    let base = if base_slug.is_empty() {
        "project"
    } else {
        base_slug
    };
    let mut candidate = base.to_string();
    let mut suffix = 2;
    loop {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM projects WHERE slug = ?1",
            params![candidate],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Ok(candidate);
        }
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
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
    fn test_schema_version_is_initialized() {
        let db = test_db();
        let conn = db.lock_conn().unwrap();
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_unversioned_existing_database_migrates() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE bookmarks (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                favicon TEXT,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            INSERT INTO settings (key, value) VALUES ('session_tabs', '[]');
        ",
        )
        .unwrap();
        let db = Db {
            conn: Mutex::new(conn),
        };

        db.init_schema().unwrap();

        let conn = db.lock_conn().unwrap();
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();
        let session_tabs: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'session_tabs'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
        assert_eq!(session_tabs, "[]");
    }

    #[test]
    fn test_lock_conn_recovers_poisoned_mutex() {
        let db = test_db();
        let _ = std::panic::catch_unwind(|| {
            let _guard = db.conn.lock().unwrap();
            panic!("poison test database lock");
        });

        let conn = db.lock_conn().unwrap();
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
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
    fn test_trim_history_keeps_most_recent_entries() {
        let db = test_db();
        {
            let mut conn = db.lock_conn().unwrap();
            let tx = conn.transaction().unwrap();
            for index in 0..(MAX_HISTORY_ENTRIES + 5) {
                tx.execute(
                    "INSERT INTO history (url, title, last_visited) VALUES (?1, ?2, ?3)",
                    params![
                        format!("https://example-{index}.com"),
                        format!("Example {index}"),
                        index,
                    ],
                )
                .unwrap();
            }
            tx.commit().unwrap();
        }

        db.trim_history().unwrap();

        let conn = db.lock_conn().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
            .unwrap();
        let oldest_kept: i64 = conn
            .query_row("SELECT MIN(last_visited) FROM history", [], |row| {
                row.get(0)
            })
            .unwrap();
        let newest_kept: i64 = conn
            .query_row("SELECT MAX(last_visited) FROM history", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(count, MAX_HISTORY_ENTRIES);
        assert_eq!(oldest_kept, 5);
        assert_eq!(newest_kept, MAX_HISTORY_ENTRIES + 4);
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

    #[test]
    fn test_project_schema_v2_is_initialized() {
        let db = test_db();
        let conn = db.lock_conn().unwrap();
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();
        let project_tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('projects', 'project_tabs', 'project_domains', 'project_events')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 2);
        assert_eq!(project_tables, 4);
    }

    #[test]
    fn test_create_open_update_project_preserves_tabs_domains_and_active_tab() {
        let db = test_db();
        let tabs = vec![
            crate::browser::TabInfo {
                id: "repo".into(),
                url: "https://github.com/nodaysidle/orbit-browser".into(),
                title: "nodaysidle/orbit-browser".into(),
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
            crate::browser::TabInfo {
                id: "local".into(),
                url: "http://localhost:3000".into(),
                title: "localhost:3000".into(),
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
            crate::browser::TabInfo {
                id: "docs".into(),
                url: "https://tauri.app/start".into(),
                title: "Tauri docs".into(),
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
        ];

        let project = db
            .create_project_from_tabs("Orbit Browser", &tabs, Some("local"), "manual")
            .unwrap();
        assert_eq!(project.name, "Orbit Browser");
        assert_eq!(project.tabs.len(), 3);
        assert_eq!(
            project.tabs[0].url,
            "https://github.com/nodaysidle/orbit-browser"
        );
        assert_eq!(project.tabs[1].is_active, true);
        assert!(project.domains.contains(&"github.com".to_string()));
        assert!(project.domains.contains(&"localhost".to_string()));

        let opened = db.open_project(&project.id).unwrap();
        assert_eq!(opened.tabs.iter().position(|tab| tab.is_active), Some(1));
        assert_eq!(opened.tabs[2].url, "https://tauri.app/start");

        let updated = db
            .update_project_from_tabs(&project.id, &tabs[..2], Some("repo"))
            .unwrap();
        assert_eq!(updated.tabs.len(), 2);
        assert_eq!(updated.tabs[0].is_active, true);
    }

    #[test]
    fn test_detected_project_from_github_localhost_and_docs_upserts_orbit_browser() {
        let db = test_db();
        let tabs = vec![
            crate::browser::TabInfo {
                id: "repo".into(),
                url: "https://github.com/nodaysidle/orbit-browser".into(),
                title: "Repo".into(),
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
            crate::browser::TabInfo {
                id: "local".into(),
                url: "http://localhost:3000".into(),
                title: "Local dev".into(),
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
            crate::browser::TabInfo {
                id: "docs".into(),
                url: "https://docs.rs/tauri/latest/tauri/".into(),
                title: "docs.rs tauri".into(),
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
        ];

        let suggestion = db.detect_project_suggestion(&tabs).unwrap();
        assert_eq!(suggestion.name, "Orbit Browser");
        assert_eq!(suggestion.confidence, 95);

        let saved = db
            .save_detected_project(&tabs, Some("repo"))
            .unwrap()
            .unwrap();
        assert_eq!(saved.name, "Orbit Browser");
        assert_eq!(saved.tabs.len(), 3);
        assert_eq!(db.get_projects().unwrap().len(), 1);

        let saved_again = db
            .save_detected_project(&tabs[..2], Some("local"))
            .unwrap()
            .unwrap();
        assert_eq!(saved_again.id, saved.id);
        assert_eq!(saved_again.tabs.len(), 2);
        assert_eq!(db.get_projects().unwrap().len(), 1);
    }

    #[test]
    fn test_detected_project_survives_close_reopen_and_restores_order() {
        let path =
            std::env::temp_dir().join(format!("orbit-project-v1-{}.db", uuid::Uuid::new_v4()));
        let tabs = vec![
            crate::browser::TabInfo {
                id: "repo".into(),
                url: "https://github.com/nodaysidle/orbit-browser".into(),
                title: "Repo".into(),
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
            crate::browser::TabInfo {
                id: "local".into(),
                url: "http://localhost:3000".into(),
                title: "Local dev".into(),
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
            crate::browser::TabInfo {
                id: "tauri".into(),
                url: "https://tauri.app/start".into(),
                title: "Tauri".into(),
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
            crate::browser::TabInfo {
                id: "docs".into(),
                url: "https://docs.rs/tauri/latest/tauri/".into(),
                title: "docs.rs tauri".into(),
                loading: false,
                can_go_back: false,
                can_go_forward: false,
            },
        ];

        let saved_id = {
            let db = Db::open(&path).unwrap();
            let saved = db
                .save_detected_project(&tabs, Some("tauri"))
                .unwrap()
                .unwrap();
            assert_eq!(saved.name, "Orbit Browser");
            saved.id
        };

        let reopened = Db::open(&path).unwrap();
        let projects = reopened.get_projects().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, saved_id);
        assert_eq!(projects[0].name, "Orbit Browser");

        let project = reopened.open_project(&saved_id).unwrap();
        let urls: Vec<_> = project.tabs.iter().map(|tab| tab.url.as_str()).collect();
        assert_eq!(
            urls,
            vec![
                "https://github.com/nodaysidle/orbit-browser",
                "http://localhost:3000",
                "https://tauri.app/start",
                "https://docs.rs/tauri/latest/tauri/"
            ]
        );
        assert_eq!(project.tabs.iter().position(|tab| tab.is_active), Some(2));
        assert!(project.domains.contains(&"github.com".to_string()));
        assert!(project.domains.contains(&"localhost".to_string()));
        assert!(project.domains.contains(&"tauri.app".to_string()));
        assert!(project.domains.contains(&"docs.rs".to_string()));

        let _ = std::fs::remove_file(path);
    }

    fn project_tab(id: &str, url: &str, title: &str) -> crate::browser::TabInfo {
        crate::browser::TabInfo {
            id: id.into(),
            url: url.into(),
            title: title.into(),
            loading: false,
            can_go_back: false,
            can_go_forward: false,
        }
    }

    #[test]
    fn test_active_project_session_save_updates_snapshot_and_restore_guard_blocks_transitions() {
        let db = test_db();
        let initial_tabs = vec![
            project_tab(
                "repo",
                "https://github.com/nodaysidle/orbit-browser",
                "Orbit repo",
            ),
            project_tab("local", "http://localhost:3000", "Orbit local"),
        ];
        let project = db
            .create_project_from_tabs("Orbit Browser", &initial_tabs, Some("repo"), "manual")
            .unwrap();
        db.set_setting("active_project_id", &project.id).unwrap();

        let updated_tabs = vec![
            project_tab(
                "repo",
                "https://github.com/nodaysidle/orbit-browser",
                "Orbit repo",
            ),
            project_tab("docs", "https://docs.rs/tauri/latest/tauri/", "Tauri docs"),
        ];
        db.save_active_project_snapshot_from_settings(&updated_tabs, Some("docs"))
            .unwrap();
        let updated = db.open_project(&project.id).unwrap();
        assert_eq!(updated.tabs.len(), 2);
        assert_eq!(updated.tabs[1].url, "https://docs.rs/tauri/latest/tauri/");
        assert_eq!(updated.tabs.iter().position(|tab| tab.is_active), Some(1));

        db.set_setting("project_restore_started_at", &unix_timestamp().to_string())
            .unwrap();
        let transitional_tabs = vec![project_tab("blank", "", "New Tab")];
        db.save_active_project_snapshot_from_settings(&transitional_tabs, Some("blank"))
            .unwrap();
        let guarded = db.open_project(&project.id).unwrap();
        assert_eq!(guarded.tabs.len(), 2);
        assert_eq!(guarded.tabs[1].url, "https://docs.rs/tauri/latest/tauri/");
    }

    #[test]
    fn test_three_named_builder_projects_create_save_reopen_resume_and_archive() {
        let path =
            std::env::temp_dir().join(format!("orbit-project-v15-{}.db", uuid::Uuid::new_v4()));
        let specs = [
            (
                "Orbit Browser",
                vec![
                    project_tab(
                        "orbit-repo",
                        "https://github.com/nodaysidle/orbit-browser",
                        "Orbit repo",
                    ),
                    project_tab("orbit-local", "http://localhost:3000", "Orbit local"),
                    project_tab("orbit-docs", "https://tauri.app/start", "Tauri"),
                ],
                "orbit-local",
            ),
            (
                "EchoCorePro",
                vec![
                    project_tab(
                        "echo-repo",
                        "https://github.com/nodaysidle/echocorepro",
                        "Echo repo",
                    ),
                    project_tab("echo-local", "http://localhost:5173", "Echo local"),
                    project_tab(
                        "echo-mdn",
                        "https://developer.mozilla.org/en-US/docs/Web/API",
                        "MDN",
                    ),
                ],
                "echo-repo",
            ),
            (
                "Synapse-Notes",
                vec![
                    project_tab(
                        "synapse-repo",
                        "https://github.com/nodaysidle/synapse-notes",
                        "Synapse repo",
                    ),
                    project_tab("synapse-local", "http://localhost:1420", "Synapse local"),
                    project_tab(
                        "synapse-docs",
                        "https://docs.rs/tauri/latest/tauri/",
                        "Tauri docs",
                    ),
                ],
                "synapse-docs",
            ),
        ];

        let ids: Vec<String> = {
            let db = Db::open(&path).unwrap();
            let created: Vec<String> = specs
                .iter()
                .map(|(name, tabs, active)| {
                    db.create_project_from_tabs(name, tabs, Some(active), "manual")
                        .unwrap()
                        .id
                })
                .collect();
            for ((_, tabs, _), id) in specs.iter().zip(created.iter()) {
                let mut updated_tabs = tabs.clone();
                updated_tabs.push(project_tab(
                    "update-docs",
                    "https://stackoverflow.com/questions/tagged/tauri",
                    "Tauri questions",
                ));
                db.update_project_from_tabs(id, &updated_tabs, Some("update-docs"))
                    .unwrap();
            }
            created
        };

        let reopened = Db::open(&path).unwrap();
        assert_eq!(reopened.get_projects().unwrap().len(), 3);
        for ((name, tabs, _active), id) in specs.iter().zip(ids.iter()) {
            let project = reopened.open_project(id).unwrap();
            assert_eq!(&project.name, name);
            assert_eq!(project.tabs.len(), tabs.len() + 1);
            for (index, expected) in tabs.iter().enumerate() {
                assert_eq!(project.tabs[index].position, index as i64);
                assert_eq!(project.tabs[index].url, expected.url);
            }
            let updated_index = tabs.len();
            assert_eq!(
                project.tabs[updated_index].url,
                "https://stackoverflow.com/questions/tagged/tauri"
            );
            assert_eq!(
                project.tabs.iter().position(|tab| tab.is_active),
                Some(updated_index)
            );
        }

        reopened.delete_project(&ids[1]).unwrap();
        let mut remaining: Vec<String> = reopened
            .get_projects()
            .unwrap()
            .into_iter()
            .map(|project| project.name)
            .collect();
        remaining.sort();
        assert_eq!(
            remaining,
            vec!["Orbit Browser".to_string(), "Synapse-Notes".to_string()]
        );

        let _ = std::fs::remove_file(path);
    }
}
