# Orbit Browser — Full Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a complete, working Tauri 2 browser with native child webviews, SQLite persistence, ad blocking, and a polished graphite/blue design.

**Architecture:** Main window hosts the chrome UI webview (108px chrome). Each tab is a native child `Webview` added via `main_window.add_child()`, positioned at y=108 below the chrome. Tabs show/hide with `set_visible()`. Rust state tracks all tab data, emitting events to the frontend on page load changes.

**Tech Stack:** Tauri 2.10.3, Rust, rusqlite (bundled), urlencoding, Space Grotesk + JetBrains Mono (self-hosted woff2), Vite (frontend bundler), vanilla JS with `@tauri-apps/api`

**Working directory:** `/Volumes/omarchyuser/minimal-browser/orbit-browser`

---

## Task 1: Dependencies + Directory Structure

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/browser.rs` (empty)
- Create: `src-tauri/src/db.rs` (empty)
- Create: `src-tauri/src/adblock.rs` (empty)
- Create: `src/fonts/` directory

**Step 1: Update Cargo.toml**

Replace the entire `[dependencies]` section:

```toml
[package]
name = "orbit"
version = "1.0.0"
description = "Orbit Browser"
authors = []
edition = "2021"
rust-version = "1.75"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v4"] }
url = "2"
rusqlite = { version = "0.31", features = ["bundled"] }
urlencoding = "2"
```

**Step 2: Create empty module files**

```bash
touch src-tauri/src/browser.rs src-tauri/src/db.rs src-tauri/src/adblock.rs
mkdir -p src/fonts
```

**Step 3: Download fonts**

```bash
# Space Grotesk (3 weights needed: 400, 500, 600)
curl -L "https://github.com/floriankarsten/space-grotesk/raw/master/fonts/webfonts/SpaceGrotesk-Regular.woff2" -o src/fonts/SpaceGrotesk-Regular.woff2
curl -L "https://github.com/floriankarsten/space-grotesk/raw/master/fonts/webfonts/SpaceGrotesk-Medium.woff2" -o src/fonts/SpaceGrotesk-Medium.woff2
curl -L "https://github.com/floriankarsten/space-grotesk/raw/master/fonts/webfonts/SpaceGrotesk-SemiBold.woff2" -o src/fonts/SpaceGrotesk-SemiBold.woff2

# JetBrains Mono (Regular weight only for the wordmark)
curl -L "https://github.com/JetBrains/JetBrainsMono/raw/master/fonts/webfonts/JetBrainsMono-Regular.woff2" -o src/fonts/JetBrainsMono-Regular.woff2
```

**Step 4: Verify fonts downloaded**

```bash
ls -lh src/fonts/
# Should show 4 .woff2 files, each 30-80KB
```

**Step 5: Verify Cargo compiles**

```bash
cd src-tauri && cargo build 2>&1 | tail -20
# Should compile clean (browser.rs/db.rs/adblock.rs are empty for now)
```

Note: `rusqlite --features bundled` compiles SQLite from source. First build takes ~2 minutes.

**Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/browser.rs src-tauri/src/db.rs src-tauri/src/adblock.rs src/fonts/
git commit -m "chore: add dependencies and font assets"
```

---

## Task 2: URL Normalization (with tests)

**Files:**
- Modify: `src-tauri/src/browser.rs`

**Step 1: Write the failing test**

Add to `src-tauri/src/browser.rs`:

```rust
pub fn normalize_url(input: &str) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_full_url_passthrough() {
        assert_eq!(normalize_url("https://github.com"), "https://github.com");
        assert_eq!(normalize_url("http://localhost:3000"), "http://localhost:3000");
    }

    #[test]
    fn test_normalize_adds_https() {
        assert_eq!(normalize_url("github.com"), "https://github.com");
        assert_eq!(normalize_url("  github.com  "), "https://github.com");
    }

    #[test]
    fn test_normalize_search_query() {
        assert_eq!(
            normalize_url("how to center a div"),
            "https://duckduckgo.com/?q=how%20to%20center%20a%20div"
        );
    }

    #[test]
    fn test_normalize_search_preserves_dotless_input() {
        assert_eq!(
            normalize_url("rust ownership"),
            "https://duckduckgo.com/?q=rust%20ownership"
        );
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test normalize 2>&1 | tail -10
# Expected: FAIL with "not yet implemented"
```

**Step 3: Implement normalize_url**

Replace `todo!()` with:

```rust
pub fn normalize_url(input: &str) -> String {
    let s = input.trim();
    if s.starts_with("http://") || s.starts_with("https://") {
        s.to_string()
    } else if s.contains('.') && !s.contains(' ') && !s.is_empty() {
        format!("https://{s}")
    } else {
        format!("https://duckduckgo.com/?q={}", urlencoding::encode(s))
    }
}
```

Also add a `title_from_url` helper below it:

```rust
pub fn title_from_url(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_string()))
        .unwrap_or_else(|| "New Tab".to_string())
}
```

**Step 4: Run tests to verify they pass**

```bash
cd src-tauri && cargo test normalize 2>&1 | tail -10
# Expected: test result: ok. 4 passed
```

**Step 5: Commit**

```bash
git add src-tauri/src/browser.rs
git commit -m "feat: add URL normalization with tests"
```

---

## Task 3: Ad Blocker

**Files:**
- Modify: `src-tauri/src/adblock.rs`
- Reference: `resources/adblock-patterns.json` (39 domains already exist)

**Step 1: Write failing test**

```rust
use std::collections::HashSet;
use std::sync::Arc;

pub struct AdBlocker {
    pub blocked_domains: Arc<HashSet<String>>,
}

impl AdBlocker {
    pub fn new(domains: Vec<String>) -> Self {
        Self {
            blocked_domains: Arc::new(domains.into_iter().collect()),
        }
    }

    pub fn is_blocked(&self, url: &url::Url) -> bool {
        todo!()
    }

    pub fn arc_domains(&self) -> Arc<HashSet<String>> {
        self.blocked_domains.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocker() -> AdBlocker {
        AdBlocker::new(vec![
            "doubleclick.net".into(),
            "googleadservices.com".into(),
            "ads.example.com".into(),
        ])
    }

    #[test]
    fn test_blocks_exact_domain() {
        let b = blocker();
        let url = url::Url::parse("https://doubleclick.net/ad").unwrap();
        assert!(b.is_blocked(&url));
    }

    #[test]
    fn test_blocks_subdomain() {
        let b = blocker();
        let url = url::Url::parse("https://cdn.doubleclick.net/pixel.gif").unwrap();
        assert!(b.is_blocked(&url));
    }

    #[test]
    fn test_allows_clean_domain() {
        let b = blocker();
        let url = url::Url::parse("https://github.com").unwrap();
        assert!(!b.is_blocked(&url));
    }

    #[test]
    fn test_allows_domain_that_contains_blocked_as_substring() {
        // "notdoubleclick.net" should NOT be blocked
        let b = blocker();
        let url = url::Url::parse("https://notdoubleclick.net").unwrap();
        assert!(!b.is_blocked(&url));
    }
}
```

**Step 2: Run tests to fail**

```bash
cd src-tauri && cargo test adblock 2>&1 | tail -10
```

**Step 3: Implement is_blocked**

```rust
pub fn is_blocked(&self, url: &url::Url) -> bool {
    let host = match url.host_str() {
        Some(h) => h,
        None => return false,
    };
    // Check exact match and subdomain match
    if self.blocked_domains.contains(host) {
        return true;
    }
    // Check if host ends with .blocked_domain
    for domain in self.blocked_domains.iter() {
        if host.ends_with(&format!(".{domain}")) {
            return true;
        }
    }
    false
}
```

Also add a loader function:

```rust
pub fn load_from_json(json_path: &std::path::Path) -> Self {
    let content = std::fs::read_to_string(json_path).unwrap_or_default();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
    let domains = parsed["domains"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Self::new(domains)
}
```

**Step 4: Run tests to pass**

```bash
cd src-tauri && cargo test adblock 2>&1 | tail -10
# Expected: test result: ok. 4 passed
```

**Step 5: Commit**

```bash
git add src-tauri/src/adblock.rs
git commit -m "feat: add ad blocker with subdomain matching"
```

---

## Task 4: Database Layer

**Files:**
- Modify: `src-tauri/src/db.rs`

**Step 1: Write the full db.rs with schema + failing tests**

```rust
use rusqlite::{Connection, Result, params};
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
    fn test_bookmark_add_get_delete() {
        let db = test_db();
        let bm = db.add_bookmark("https://github.com", "GitHub").unwrap();
        assert_eq!(bm.url, "https://github.com");

        let all = db.get_bookmarks().unwrap();
        assert_eq!(all.len(), 1);
        assert!(db.is_bookmarked("https://github.com").unwrap());

        db.delete_bookmark(&bm.id).unwrap();
        assert!(!db.is_bookmarked("https://github.com").unwrap());
    }

    #[test]
    fn test_history_upsert_increments_count() {
        let db = test_db();
        db.add_history("https://rust-lang.org", "Rust").unwrap();
        db.add_history("https://rust-lang.org", "Rust").unwrap();
        let h = db.get_history(10, 0).unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].visit_count, 2);
    }

    #[test]
    fn test_history_search() {
        let db = test_db();
        db.add_history("https://rust-lang.org", "Rust Programming").unwrap();
        db.add_history("https://github.com", "GitHub").unwrap();
        let results = db.search_history("rust").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://rust-lang.org");
    }

    #[test]
    fn test_settings() {
        let db = test_db();
        assert!(db.get_setting("theme").unwrap().is_none());
        db.set_setting("theme", "dark").unwrap();
        assert_eq!(db.get_setting("theme").unwrap(), Some("dark".into()));
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test db:: 2>&1 | tail -15
# Expected: compile errors or test failures
```

**Step 3: Fix any compile issues and run until passing**

```bash
cd src-tauri && cargo test db:: 2>&1 | tail -15
# Expected: test result: ok. 4 passed
```

**Step 4: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat: SQLite database layer with bookmark, history, settings"
```

---

## Task 5: Browser State + Tab Types

**Files:**
- Modify: `src-tauri/src/browser.rs` (add after normalize_url)

**Step 1: Add state types to browser.rs**

Append to `src-tauri/src/browser.rs` after the existing functions:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TabInfo {
    pub id: String,
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
}

#[derive(Debug)]
pub struct TabData {
    pub info: TabInfo,
    pub history: Vec<String>,   // URL history stack
    pub history_idx: usize,     // Current position in history
    pub has_webview: bool,      // false for blank tabs not yet navigated
}

#[derive(Default)]
pub struct BrowserState {
    pub tabs: Mutex<HashMap<String, TabData>>,
    pub active_tab: Mutex<Option<String>>,
}

impl BrowserState {
    pub fn new() -> Self {
        Self::default()
    }
}
```

**Step 2: Add test for history tracking logic**

Append test to the existing `#[cfg(test)]` block in `browser.rs`:

```rust
#[test]
fn test_history_tracking_back_detection() {
    let mut history = vec!["https://a.com".to_string(), "https://b.com".to_string()];
    let mut idx: usize = 1;

    // Simulate navigating "back" — browser goes to a.com
    let new_url = "https://a.com";
    if idx > 0 && history[idx - 1] == new_url {
        idx -= 1;
    } else {
        history.truncate(idx + 1);
        history.push(new_url.to_string());
        idx = history.len() - 1;
    }

    assert_eq!(idx, 0);
    assert_eq!(history.len(), 2); // Not pushed again
}
```

**Step 3: Run test**

```bash
cd src-tauri && cargo test history_tracking 2>&1 | tail -5
```

**Step 4: Commit**

```bash
git add src-tauri/src/browser.rs
git commit -m "feat: browser state types and tab data structures"
```

---

## Task 6: Tab Management Commands

**Files:**
- Modify: `src-tauri/src/main.rs` (full rewrite)

**Step 1: Replace main.rs entirely**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod browser;
mod db;
mod adblock;

use browser::{normalize_url, title_from_url, BrowserState, TabData, TabInfo};
use db::Db;
use adblock::AdBlocker;

use std::sync::Arc;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager,
    WebviewUrl, Rect, Position, Size,
};
use tauri::webview::WebviewBuilder;

const CHROME_HEIGHT: f64 = 108.0;

fn get_logical_size(app: &AppHandle) -> (f64, f64) {
    let win = app.get_webview_window("main").expect("main window");
    let size = win.inner_size().unwrap_or_default();
    let scale = win.scale_factor().unwrap_or(1.0);
    (size.width as f64 / scale, size.height as f64 / scale)
}

// ── Tab Commands ──────────────────────────────────────────────────────────────

#[tauri::command]
async fn create_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    url: String,
    make_active: bool,
) -> Result<TabInfo, String> {
    let id = format!("t{}", &uuid::Uuid::new_v4().simple().to_string()[..10]);
    let is_blank = url.trim().is_empty() || url.trim() == "about:blank";

    let tab_info = TabInfo {
        id: id.clone(),
        url: if is_blank { String::new() } else { normalize_url(&url) },
        title: "New Tab".to_string(),
        loading: false,
        can_go_back: false,
        can_go_forward: false,
    };

    {
        let mut tabs = state.tabs.lock().unwrap();
        tabs.insert(id.clone(), TabData {
            info: tab_info.clone(),
            history: Vec::new(),
            history_idx: 0,
            has_webview: false,
        });
    }

    if make_active {
        *state.active_tab.lock().unwrap() = Some(id.clone());
    }

    // If URL provided, actually create the webview
    if !is_blank {
        create_webview_for_tab(&app, &state, &id, &normalize_url(&url), make_active)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(tab_info)
}

async fn create_webview_for_tab(
    app: &AppHandle,
    state: &tauri::State<'_, BrowserState>,
    tab_id: &str,
    url: &str,
    visible: bool,
) -> Result<(), String> {
    let webview_url = WebviewUrl::External(
        url.parse().map_err(|e: url::ParseError| e.to_string())?
    );
    let (lw, lh) = get_logical_size(app);
    let main = app.get_webview_window("main").ok_or("no main window")?;

    let id_c = tab_id.to_string();
    let app_c = app.clone();
    let app_nav = app.clone();
    let id_nav = tab_id.to_string();

    // Clone blocked domains Arc for the on_navigation closure
    let blocked = app.state::<AdBlocker>().arc_domains();

    main.add_child(
        WebviewBuilder::new(tab_id, webview_url)
            .visible(visible)
            .on_navigation(move |nav_url| {
                // Block ad domains
                if let Some(host) = nav_url.host_str() {
                    if blocked.contains(host) {
                        return false;
                    }
                    for domain in blocked.iter() {
                        if host.ends_with(&format!(".{domain}")) {
                            return false;
                        }
                    }
                }
                // Emit navigation start to update address bar immediately
                let _ = app_nav.emit("tab-navigating", serde_json::json!({
                    "id": id_nav,
                    "url": nav_url.to_string()
                }));
                true
            })
            .on_page_load(move |_, payload| {
                use tauri::webview::PageLoadEvent;
                let url_str = payload.url().to_string();
                let browser_state = app_c.state::<BrowserState>();
                let db = app_c.state::<Db>();

                match payload.event() {
                    PageLoadEvent::Started => {
                        let mut tabs = browser_state.tabs.lock().unwrap();
                        if let Some(td) = tabs.get_mut(&id_c) {
                            td.info.loading = true;
                            td.info.url = url_str.clone();
                        }
                        let _ = app_c.emit("tab-loading", serde_json::json!({
                            "id": id_c,
                            "url": url_str
                        }));
                    }
                    PageLoadEvent::Finished => {
                        let title = title_from_url(&url_str);
                        let info = {
                            let mut tabs = browser_state.tabs.lock().unwrap();
                            let Some(td) = tabs.get_mut(&id_c) else { return };
                            // Update history stack
                            if td.history_idx > 0 && td.history.get(td.history_idx - 1).map(|u| u == &url_str).unwrap_or(false) {
                                td.history_idx -= 1;
                            } else if td.history_idx + 1 < td.history.len() && td.history[td.history_idx + 1] == url_str {
                                td.history_idx += 1;
                            } else if td.history.last().map(|u| u != &url_str).unwrap_or(true) {
                                td.history.truncate(td.history_idx + 1);
                                td.history.push(url_str.clone());
                                td.history_idx = td.history.len() - 1;
                            }
                            td.info.loading = false;
                            td.info.url = url_str.clone();
                            td.info.title = title.clone();
                            td.info.can_go_back = td.history_idx > 0;
                            td.info.can_go_forward = td.history_idx + 1 < td.history.len();
                            td.info.clone()
                        };
                        // Persist to history (non-blocking)
                        if !url_str.starts_with("about:") {
                            let _ = db.add_history(&url_str, &title);
                        }
                        let _ = app_c.emit("tab-loaded", &info);
                    }
                }
            }),
        LogicalPosition::new(0.0, CHROME_HEIGHT),
        LogicalSize::new(lw, lh - CHROME_HEIGHT),
    ).map_err(|e| e.to_string())?;

    // Mark webview as created in state
    let mut tabs = state.tabs.lock().unwrap();
    if let Some(td) = tabs.get_mut(tab_id) {
        td.has_webview = true;
    }

    Ok(())
}

#[tauri::command]
async fn switch_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
) -> Result<(), String> {
    let (lw, lh) = get_logical_size(&app);
    let main = app.get_webview_window("main").ok_or("no main")?;
    let tabs = state.tabs.lock().unwrap();

    for id in tabs.keys() {
        if let Some(wv) = app.get_webview(id) {
            let visible = *id == tab_id;
            let _ = wv.set_visible(visible);
            if visible {
                let _ = wv.set_bounds(Rect {
                    position: Position::Logical(LogicalPosition::new(0.0, CHROME_HEIGHT)),
                    size: Size::Logical(LogicalSize::new(lw, lh - CHROME_HEIGHT)),
                });
            }
        }
    }
    drop(tabs);

    *state.active_tab.lock().unwrap() = Some(tab_id);
    Ok(())
}

#[tauri::command]
async fn close_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
) -> Result<Option<String>, String> {
    // Close native webview if it exists
    if let Some(wv) = app.get_webview(&tab_id) {
        wv.close().map_err(|e| e.to_string())?;
    }

    let mut tabs = state.tabs.lock().unwrap();
    tabs.remove(&tab_id);

    let mut active = state.active_tab.lock().unwrap();
    if active.as_deref() == Some(&tab_id) {
        let new_id = tabs.keys().next().cloned();
        *active = new_id.clone();
        return Ok(new_id);
    }
    Ok(active.clone())
}

#[tauri::command]
async fn navigate_tab(
    app: AppHandle,
    state: tauri::State<'_, BrowserState>,
    tab_id: String,
    url: String,
) -> Result<(), String> {
    let clean = normalize_url(&url);
    let has_webview = state.tabs.lock().unwrap()
        .get(&tab_id).map(|t| t.has_webview).unwrap_or(false);

    if has_webview {
        if let Some(wv) = app.get_webview(&tab_id) {
            wv.navigate(clean.parse().map_err(|e: url::ParseError| e.to_string())?)
                .map_err(|e| e.to_string())?;
        }
    } else {
        // Lazy creation — first navigation on a blank tab
        let make_active = state.active_tab.lock().unwrap().as_deref() == Some(&tab_id);
        create_webview_for_tab(&app, &state, &tab_id, &clean, make_active).await?;
    }
    Ok(())
}

#[tauri::command]
async fn go_back(app: AppHandle, state: tauri::State<'_, BrowserState>, tab_id: String) -> Result<(), String> {
    let can = state.tabs.lock().unwrap()
        .get(&tab_id).map(|t| t.history_idx > 0).unwrap_or(false);
    if can {
        if let Some(wv) = app.get_webview(&tab_id) {
            let _ = wv.eval("history.back()");
        }
    }
    Ok(())
}

#[tauri::command]
async fn go_forward(app: AppHandle, state: tauri::State<'_, BrowserState>, tab_id: String) -> Result<(), String> {
    let can = state.tabs.lock().unwrap()
        .get(&tab_id).map(|t| {
            let td = t;
            td.history_idx + 1 < td.history.len()
        }).unwrap_or(false);
    if can {
        if let Some(wv) = app.get_webview(&tab_id) {
            let _ = wv.eval("history.forward()");
        }
    }
    Ok(())
}

#[tauri::command]
async fn reload_tab(app: AppHandle, tab_id: String) -> Result<(), String> {
    if let Some(wv) = app.get_webview(&tab_id) {
        let _ = wv.eval("location.reload()");
    }
    Ok(())
}

#[tauri::command]
fn get_tabs(state: tauri::State<'_, BrowserState>) -> Vec<TabInfo> {
    state.tabs.lock().unwrap().values().map(|td| td.info.clone()).collect()
}

#[tauri::command]
fn get_active_tab(state: tauri::State<'_, BrowserState>) -> Option<String> {
    state.active_tab.lock().unwrap().clone()
}

// ── DB Commands ───────────────────────────────────────────────────────────────

#[tauri::command]
fn add_bookmark(db: tauri::State<'_, Db>, url: String, title: String) -> Result<db::Bookmark, String> {
    db.add_bookmark(&url, &title).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_bookmarks(db: tauri::State<'_, Db>) -> Result<Vec<db::Bookmark>, String> {
    db.get_bookmarks().map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_bookmark(db: tauri::State<'_, Db>, id: String) -> Result<(), String> {
    db.delete_bookmark(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn is_bookmarked(db: tauri::State<'_, Db>, url: String) -> Result<bool, String> {
    db.is_bookmarked(&url).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_history(db: tauri::State<'_, Db>, limit: i64, offset: i64) -> Result<Vec<db::HistoryEntry>, String> {
    db.get_history(limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_history(db: tauri::State<'_, Db>, query: String) -> Result<Vec<db::HistoryEntry>, String> {
    db.search_history(&query).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_history(db: tauri::State<'_, Db>) -> Result<(), String> {
    db.clear_history().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_setting(db: tauri::State<'_, Db>, key: String) -> Result<Option<String>, String> {
    db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_setting(db: tauri::State<'_, Db>, key: String, value: String) -> Result<(), String> {
    db.set_setting(&key, &value).map_err(|e| e.to_string())
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize database
            let db_path = app.path().app_data_dir()
                .expect("app data dir")
                .join("orbit.db");
            std::fs::create_dir_all(db_path.parent().unwrap()).ok();
            let db = Db::open(&db_path).expect("open db");
            app.manage(db);

            // Initialize ad blocker
            let blocklist_path = app.path().resource_dir()
                .expect("resource dir")
                .join("resources/adblock-patterns.json");
            let blocker = if blocklist_path.exists() {
                AdBlocker::load_from_json(&blocklist_path)
            } else {
                AdBlocker::new(vec![])
            };
            app.manage(blocker);

            // Initialize browser state
            app.manage(BrowserState::new());

            // Window resize handler — update active webview bounds
            let app_h = app.handle().clone();
            let main = app.get_webview_window("main").unwrap();
            main.on_window_event(move |event| {
                if let tauri::WindowEvent::Resized(size) = event {
                    let scale = app_h.get_webview_window("main")
                        .map(|w| w.scale_factor().unwrap_or(1.0))
                        .unwrap_or(1.0);
                    let lw = size.width as f64 / scale;
                    let lh = size.height as f64 / scale;

                    let state = app_h.state::<BrowserState>();
                    let active = state.active_tab.lock().unwrap().clone();
                    if let Some(id) = active {
                        if let Some(wv) = app_h.get_webview(&id) {
                            let _ = wv.set_bounds(Rect {
                                position: Position::Logical(LogicalPosition::new(0.0, CHROME_HEIGHT)),
                                size: Size::Logical(LogicalSize::new(lw, lh - CHROME_HEIGHT)),
                            });
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_tab, switch_tab, close_tab, navigate_tab,
            go_back, go_forward, reload_tab, get_tabs, get_active_tab,
            add_bookmark, get_bookmarks, delete_bookmark, is_bookmarked,
            get_history, search_history, clear_history,
            get_setting, set_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 2: Build to check for compile errors**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "error|warning: unused" | head -30
```

Fix any compile errors (common issues: import paths, method names on Webview).
Key things to check if errors occur:
- `app.get_webview(id)` vs `main_window.get_webview(id)` — try both
- `wv.set_bounds(Rect {...})` — check Tauri 2.10.x Rect field names
- `WebviewBuilder::visible(bool)` — may be `set_visible(bool)` or need post-creation call
- `Webview::navigate(url::Url)` — confirm signature

**Step 3: Commit once it compiles**

```bash
git add src-tauri/src/main.rs
git commit -m "feat: full Rust browser backend with tab management, navigation, DB, adblock"
```

---

## Task 7: Frontend HTML Structure

**Files:**
- Modify: `index.html`

**Step 1: Replace index.html entirely**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Orbit</title>
  <link rel="stylesheet" href="src/styles.css" />
</head>
<body>
  <div id="app">

    <!-- Titlebar -->
    <div class="titlebar" data-tauri-drag-region>
      <div class="traffic-lights">
        <button id="btnClose"    class="traffic close"    aria-label="Close"></button>
        <button id="btnMinimize" class="traffic minimize" aria-label="Minimize"></button>
        <button id="btnMaximize" class="traffic maximize" aria-label="Maximize"></button>
      </div>
      <div class="tab-count" id="tabCount">1 tab</div>
    </div>

    <!-- Tab Bar -->
    <div class="tab-bar">
      <div class="tabs-scroll" id="tabsContainer"></div>
      <button class="new-tab-btn" id="btnNewTab" aria-label="New tab">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
          <line x1="6" y1="1" x2="6" y2="11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="1" y1="6" x2="11" y2="6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
    </div>

    <!-- Nav Bar -->
    <div class="nav-bar">
      <div class="nav-left">
        <button id="btnBack"   class="nav-btn" aria-label="Back" disabled>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M9 11L5 7l4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
        <button id="btnForward" class="nav-btn" aria-label="Forward" disabled>
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M5 3l4 4-4 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
        <button id="btnReload" class="nav-btn" aria-label="Reload">
          <svg id="iconReload" width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M12 2v4H8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M2 12V8h4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M11.5 6A5 5 0 0 0 3 3.5L2 5M2.5 8a5 5 0 0 0 8.5 2.5l1-1.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <div class="address-wrap">
        <div class="lock-icon" id="lockIcon">
          <svg width="10" height="12" viewBox="0 0 10 12" fill="none">
            <rect x="1" y="5" width="8" height="6" rx="1.5" stroke="currentColor" stroke-width="1.2"/>
            <path d="M3 5V3.5a2 2 0 0 1 4 0V5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          </svg>
        </div>
        <input id="addressInput" type="text" placeholder="Search or enter address" autocomplete="off" spellcheck="false" />
      </div>

      <div class="nav-right">
        <button id="btnBookmark" class="nav-btn" aria-label="Bookmark">
          <svg id="iconBookmark" width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M3 2h8a1 1 0 0 1 1 1v9l-5-3-5 3V3a1 1 0 0 1 1-1z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>
          </svg>
        </button>
        <button id="btnMenu" class="nav-btn" aria-label="Menu">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <circle cx="7" cy="3" r="1" fill="currentColor"/>
            <circle cx="7" cy="7" r="1" fill="currentColor"/>
            <circle cx="7" cy="11" r="1" fill="currentColor"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- New Tab Page (shown when no webview active) -->
    <div class="new-tab-page" id="newTabPage">
      <div class="wordmark">nodaysidle</div>
      <div class="wordmark-rule"></div>
      <div class="shortcuts-row">

        <button class="shortcut-btn" data-url="https://gitlab.com/NODAYSIDLE" data-label="GitLab">
          <svg width="22" height="22" viewBox="0 0 22 22" fill="none">
            <path d="M11 19.5L6.5 6 3 11l8 8.5z" fill="currentColor" opacity="0.9"/>
            <path d="M11 19.5L15.5 6 19 11l-8 8.5z" fill="currentColor" opacity="0.7"/>
            <path d="M6.5 6h9L11 19.5 6.5 6z" fill="currentColor"/>
          </svg>
          <span class="shortcut-tooltip">gitlab.com/NODAYSIDLE</span>
        </button>

        <button class="shortcut-btn" data-url="https://youtube.com" data-label="YouTube">
          <svg width="22" height="22" viewBox="0 0 22 22" fill="none">
            <rect x="2" y="5" width="18" height="12" rx="3" stroke="currentColor" stroke-width="1.4"/>
            <path d="M9 8.5l5 2.5-5 2.5V8.5z" fill="currentColor"/>
          </svg>
          <span class="shortcut-tooltip">YouTube</span>
        </button>

        <button class="shortcut-btn" data-url="https://producthunt.com" data-label="ProductHunt">
          <svg width="22" height="22" viewBox="0 0 22 22" fill="none">
            <circle cx="11" cy="11" r="9" stroke="currentColor" stroke-width="1.4"/>
            <path d="M8 7h4a3 3 0 0 1 0 6H8V7z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/>
            <line x1="8" y1="13" x2="8" y2="16" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
          </svg>
          <span class="shortcut-tooltip">Product Hunt</span>
        </button>

        <button class="shortcut-btn" data-url="https://web.telegram.org" data-label="Telegram">
          <svg width="22" height="22" viewBox="0 0 22 22" fill="none">
            <circle cx="11" cy="11" r="9" stroke="currentColor" stroke-width="1.4"/>
            <path d="M6 11l3 3 7-7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <span class="shortcut-tooltip">Telegram Web</span>
        </button>

      </div>
    </div>

    <!-- Dropdown menus (hidden by default) -->
    <div class="dropdown hidden" id="menuDropdown">
      <button class="dropdown-item" id="menuHistory">History</button>
      <button class="dropdown-item" id="menuBookmarks">Bookmarks</button>
      <button class="dropdown-item" id="menuClearHistory">Clear History</button>
    </div>

    <div class="dropdown panel hidden" id="historyPanel">
      <div class="panel-header">
        <span>History</span>
        <button class="panel-close" id="closeHistory">✕</button>
      </div>
      <input class="panel-search" id="historySearch" placeholder="Search history…" />
      <div class="panel-list" id="historyList"></div>
    </div>

    <div class="dropdown panel hidden" id="bookmarksPanel">
      <div class="panel-header">
        <span>Bookmarks</span>
        <button class="panel-close" id="closeBookmarks">✕</button>
      </div>
      <div class="panel-list" id="bookmarksList"></div>
    </div>

  </div>

  <script type="module" src="src/main.js"></script>
</body>
</html>
```

**Step 2: Commit**

```bash
git add index.html
git commit -m "feat: browser chrome HTML structure"
```

---

## Task 8: CSS Styles

**Files:**
- Modify: `src/styles.css`

**Step 1: Replace styles.css entirely**

```css
/* ── Fonts ──────────────────────────────────────────────────────────────────── */
@font-face {
  font-family: 'Space Grotesk';
  src: url('./fonts/SpaceGrotesk-Regular.woff2') format('woff2');
  font-weight: 400;
  font-display: block;
}
@font-face {
  font-family: 'Space Grotesk';
  src: url('./fonts/SpaceGrotesk-Medium.woff2') format('woff2');
  font-weight: 500;
  font-display: block;
}
@font-face {
  font-family: 'Space Grotesk';
  src: url('./fonts/SpaceGrotesk-SemiBold.woff2') format('woff2');
  font-weight: 600;
  font-display: block;
}
@font-face {
  font-family: 'JetBrains Mono';
  src: url('./fonts/JetBrainsMono-Regular.woff2') format('woff2');
  font-weight: 400;
  font-display: block;
}

/* ── Reset + Root ───────────────────────────────────────────────────────────── */
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
  --bg:             #0c0c0e;
  --surface:        #131316;
  --surface-raised: #1a1a1f;
  --border:         rgba(255, 255, 255, 0.07);
  --border-active:  rgba(255, 255, 255, 0.12);
  --text:           #f0f0f2;
  --text-dim:       rgba(240, 240, 242, 0.42);
  --text-dimmer:    rgba(240, 240, 242, 0.22);
  --accent:         #4f8ef7;
  --accent-glow:    rgba(79, 142, 247, 0.18);
  --accent-dim:     rgba(79, 142, 247, 0.55);
  --danger:         #f75f4f;
  --chrome-height:  108px; /* 32 + 36 + 40 */
  --radius:         6px;
  --font-ui:        'Space Grotesk', -apple-system, sans-serif;
  --font-mono:      'JetBrains Mono', 'Fira Code', monospace;
}

html, body {
  height: 100%;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-ui);
  font-size: 13px;
  line-height: 1;
  -webkit-font-smoothing: antialiased;
  overflow: hidden;
  user-select: none;
}

button {
  font-family: var(--font-ui);
  font-size: 13px;
  border: none;
  background: none;
  cursor: pointer;
  color: inherit;
  padding: 0;
}

input {
  font-family: var(--font-ui);
  font-size: 13px;
  border: none;
  background: none;
  color: var(--text);
  outline: none;
}

#app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  position: relative;
}

/* ── Titlebar (32px) ─────────────────────────────────────────────────────────── */
.titlebar {
  height: 32px;
  display: flex;
  align-items: center;
  padding: 0 14px;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  -webkit-app-region: drag;
}

.traffic-lights {
  display: flex;
  gap: 7px;
  -webkit-app-region: no-drag;
}

.traffic {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  transition: filter 0.15s;
}
.traffic.close    { background: #ff5f57; }
.traffic.minimize { background: #febc2e; }
.traffic.maximize { background: #28c840; }
.traffic:hover    { filter: brightness(1.15); }

.tab-count {
  margin-left: auto;
  font-size: 11px;
  color: var(--text-dimmer);
  font-weight: 500;
  letter-spacing: 0.02em;
  -webkit-app-region: no-drag;
}

/* ── Tab Bar (36px) ──────────────────────────────────────────────────────────── */
.tab-bar {
  height: 36px;
  display: flex;
  align-items: center;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  padding: 0 6px;
  gap: 4px;
  flex-shrink: 0;
}

.tabs-scroll {
  flex: 1;
  display: flex;
  gap: 3px;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}
.tabs-scroll::-webkit-scrollbar { display: none; }

.tab {
  min-width: 100px;
  max-width: 200px;
  height: 28px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 10px 0 8px;
  border-radius: var(--radius);
  cursor: pointer;
  color: var(--text-dim);
  font-size: 12px;
  font-weight: 500;
  flex-shrink: 0;
  position: relative;
  transition: background 0.1s, color 0.1s;
}

.tab:hover:not(.active) {
  background: rgba(255, 255, 255, 0.04);
  color: var(--text);
}

.tab.active {
  background: var(--surface-raised);
  color: var(--text);
  box-shadow: inset 2px 0 0 var(--accent);
}

/* Loading sweep animation */
.tab.loading::after {
  content: '';
  position: absolute;
  bottom: 0; left: 0;
  height: 1.5px;
  background: var(--accent);
  animation: tab-load 1.4s ease-in-out infinite;
  border-radius: 1px;
}

@keyframes tab-load {
  0%   { width: 0%; left: 0%; }
  50%  { width: 60%; left: 20%; }
  100% { width: 0%; left: 100%; }
}

.tab-favicon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.tab-favicon img {
  width: 14px;
  height: 14px;
  border-radius: 2px;
}

.tab-monogram {
  width: 14px;
  height: 14px;
  border-radius: 3px;
  background: var(--accent-glow);
  color: var(--accent);
  font-size: 9px;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
  text-transform: uppercase;
}

.tab-title {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-close {
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 3px;
  opacity: 0;
  transition: opacity 0.15s, background 0.1s, color 0.1s;
  flex-shrink: 0;
  color: var(--text-dim);
}
.tab:hover .tab-close { opacity: 1; }
.tab-close:hover {
  background: rgba(247, 95, 79, 0.18);
  color: var(--danger);
}

.new-tab-btn {
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius);
  color: var(--text-dimmer);
  flex-shrink: 0;
  transition: background 0.1s, color 0.1s;
}
.new-tab-btn:hover {
  background: rgba(255, 255, 255, 0.06);
  color: var(--text);
}

/* ── Nav Bar (40px) ──────────────────────────────────────────────────────────── */
.nav-bar {
  height: 40px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 10px;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.nav-left, .nav-right {
  display: flex;
  gap: 2px;
  flex-shrink: 0;
}

.nav-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius);
  color: var(--text-dim);
  transition: background 0.1s, color 0.1s;
}
.nav-btn:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.06);
  color: var(--text);
}
.nav-btn:disabled { opacity: 0.28; cursor: default; }

.nav-btn.bookmarked svg path { fill: var(--accent); stroke: var(--accent); }

.address-wrap {
  flex: 1;
  height: 28px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
  background: var(--surface);
  border: 1px solid transparent;
  border-radius: var(--radius);
  transition: border-color 0.15s, background 0.15s;
}
.address-wrap:focus-within {
  border-color: var(--accent-dim);
  background: var(--surface-raised);
  box-shadow: 0 0 0 3px var(--accent-glow);
}

.lock-icon {
  color: var(--text-dimmer);
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

#addressInput {
  flex: 1;
  font-size: 12.5px;
  font-weight: 500;
  color: var(--text);
  caret-color: var(--accent);
}
#addressInput::placeholder { color: var(--text-dimmer); }
#addressInput::selection { background: var(--accent-glow); }

/* ── New Tab Page ─────────────────────────────────────────────────────────────── */
.new-tab-page {
  position: absolute;
  top: var(--chrome-height);
  left: 0; right: 0; bottom: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 20px;
  background: var(--bg);
  z-index: 10;
}

.new-tab-page.hidden { display: none; }

.wordmark {
  font-family: var(--font-mono);
  font-size: 48px;
  font-weight: 400;
  letter-spacing: -0.025em;
  color: var(--text);
  line-height: 1;
}

.wordmark-rule {
  width: 48px;
  height: 1px;
  background: var(--accent);
  border-radius: 1px;
}

.shortcuts-row {
  display: flex;
  gap: 16px;
  align-items: center;
}

.shortcut-btn {
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text-dim);
  position: relative;
  transition: color 0.15s, border-color 0.15s, box-shadow 0.15s, background 0.15s;
}
.shortcut-btn:hover {
  color: var(--text);
  border-color: var(--accent-dim);
  background: var(--surface-raised);
  box-shadow: 0 0 16px var(--accent-glow);
}

.shortcut-tooltip {
  position: absolute;
  top: calc(100% + 8px);
  left: 50%;
  transform: translateX(-50%);
  background: var(--surface-raised);
  border: 1px solid var(--border-active);
  color: var(--text-dim);
  font-size: 11px;
  font-weight: 500;
  padding: 4px 8px;
  border-radius: 4px;
  white-space: nowrap;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.15s;
  z-index: 100;
}
.shortcut-btn:hover .shortcut-tooltip { opacity: 1; }

/* ── Dropdown / Panels ────────────────────────────────────────────────────────── */
.dropdown {
  position: absolute;
  top: calc(var(--chrome-height) + 4px);
  right: 10px;
  background: var(--surface-raised);
  border: 1px solid var(--border-active);
  border-radius: 8px;
  box-shadow: 0 8px 32px rgba(0,0,0,0.5);
  z-index: 200;
  min-width: 160px;
  overflow: hidden;
}

.dropdown.panel { min-width: 300px; max-height: 420px; display: flex; flex-direction: column; }
.dropdown.hidden { display: none; }

.dropdown-item {
  display: block;
  width: 100%;
  padding: 9px 14px;
  text-align: left;
  color: var(--text-dim);
  font-size: 12.5px;
  font-weight: 500;
  transition: background 0.1s, color 0.1s;
}
.dropdown-item:hover {
  background: rgba(255,255,255,0.05);
  color: var(--text);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
  font-weight: 600;
  font-size: 12px;
  color: var(--text);
}
.panel-close {
  color: var(--text-dimmer);
  font-size: 11px;
  padding: 2px 4px;
  border-radius: 3px;
}
.panel-close:hover { background: rgba(255,255,255,0.06); color: var(--text); }

.panel-search {
  margin: 8px 10px;
  padding: 6px 10px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--text);
  font-size: 12px;
  font-family: var(--font-ui);
}
.panel-search:focus { border-color: var(--accent-dim); outline: none; }

.panel-list { overflow-y: auto; flex: 1; }

.panel-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  cursor: pointer;
  transition: background 0.1s;
}
.panel-row:hover { background: rgba(255,255,255,0.04); }

.panel-row-favicon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  border-radius: 2px;
  background: var(--surface);
  display: flex; align-items: center; justify-content: center;
  font-size: 9px;
  color: var(--text-dimmer);
}

.panel-row-text {
  flex: 1;
  min-width: 0;
}
.panel-row-title {
  font-size: 12px;
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.panel-row-url {
  font-size: 11px;
  color: var(--text-dimmer);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 1px;
}

.panel-row-delete {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 3px;
  color: var(--text-dimmer);
  font-size: 11px;
  opacity: 0;
  transition: opacity 0.1s;
}
.panel-row:hover .panel-row-delete { opacity: 1; }
.panel-row-delete:hover { background: rgba(247,95,79,0.18); color: var(--danger); }

/* ── Utilities ────────────────────────────────────────────────────────────────── */
.hidden { display: none !important; }

/* Scrollbar */
::-webkit-scrollbar { width: 4px; height: 4px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 2px; }
::-webkit-scrollbar-thumb:hover { background: rgba(255,255,255,0.2); }
```

**Step 2: Commit**

```bash
git add src/styles.css
git commit -m "feat: complete CSS design system — graphite minimal with blue accent"
```

---

## Task 9: Frontend JavaScript

**Files:**
- Modify: `src/main.js`

**Step 1: Replace src/main.js entirely**

```javascript
import { invoke } from '@tauri-apps/api/core'
import { listen }  from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

// ── State ─────────────────────────────────────────────────────────────────────
const state = {
  tabs: new Map(),   // id -> TabInfo
  activeId: null,
}

// ── DOM helpers ───────────────────────────────────────────────────────────────
const $ = id => document.getElementById(id)
const win = getCurrentWindow()

// ── Rendering ─────────────────────────────────────────────────────────────────
function renderTabs() {
  const container = $('tabsContainer')
  const activeId  = state.activeId
  const tabs      = [...state.tabs.values()]

  container.innerHTML = tabs.map(tab => `
    <div class="tab${tab.id === activeId ? ' active' : ''}${tab.loading ? ' loading' : ''}"
         data-id="${tab.id}">
      <div class="tab-favicon">${faviconHtml(tab)}</div>
      <span class="tab-title">${escHtml(tab.title || 'New Tab')}</span>
      <button class="tab-close" data-close="${tab.id}" aria-label="Close tab">
        <svg width="8" height="8" viewBox="0 0 8 8" fill="none">
          <line x1="1" y1="1" x2="7" y2="7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
          <line x1="7" y1="1" x2="1" y2="7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
  `).join('')

  // Tab count
  $('tabCount').textContent = `${tabs.length} tab${tabs.length !== 1 ? 's' : ''}`

  // Wire click events
  container.querySelectorAll('.tab').forEach(el => {
    el.addEventListener('click', e => {
      if (!e.target.closest('.tab-close')) switchTab(el.dataset.id)
    })
  })
  container.querySelectorAll('.tab-close').forEach(btn => {
    btn.addEventListener('click', e => { e.stopPropagation(); closeTab(btn.dataset.close) })
  })
}

function faviconHtml(tab) {
  if (!tab.url || !tab.url.startsWith('http')) {
    return '<div class="tab-monogram"> </div>'
  }
  const letter = (tab.title || tab.url).trim()[0]?.toUpperCase() || '?'
  const domain = (() => { try { return new URL(tab.url).hostname } catch { return '' } })()
  return `
    <img src="https://www.google.com/s2/favicons?domain=${domain}&sz=32"
         width="14" height="14"
         onerror="this.style.display='none';this.nextElementSibling.style.display='flex'"
         style="border-radius:2px" />
    <div class="tab-monogram" style="display:none">${letter}</div>
  `
}

function updateNav() {
  const tab = state.tabs.get(state.activeId)
  const url = tab?.url || ''
  $('addressInput').value  = url.startsWith('http') ? url : ''
  $('btnBack').disabled    = !tab?.can_go_back
  $('btnForward').disabled = !tab?.can_go_forward
  const isHttps = url.startsWith('https://')
  $('lockIcon').style.opacity = isHttps ? '1' : '0.3'
  updateBookmarkIcon()
  // Show/hide new tab page
  const showPage = !tab || !tab.url || !tab.url.startsWith('http')
  $('newTabPage').classList.toggle('hidden', !showPage)
}

async function updateBookmarkIcon() {
  const tab = state.tabs.get(state.activeId)
  if (!tab?.url?.startsWith('http')) return
  try {
    const marked = await invoke('is_bookmarked', { url: tab.url })
    $('btnBookmark').classList.toggle('bookmarked', marked)
  } catch {}
}

function escHtml(s) {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;')
}

// ── Actions ───────────────────────────────────────────────────────────────────
async function createTab(url = '') {
  try {
    const tab = await invoke('create_tab', { url, makeActive: true })
    state.tabs.set(tab.id, tab)
    state.activeId = tab.id
    renderTabs()
    updateNav()
    if (!url) $('addressInput').focus()
    return tab
  } catch (e) { console.error('create_tab failed', e) }
}

async function switchTab(tabId) {
  if (tabId === state.activeId) return
  try {
    await invoke('switch_tab', { tabId })
    state.activeId = tabId
    renderTabs()
    updateNav()
  } catch (e) { console.error('switch_tab failed', e) }
}

async function closeTab(tabId) {
  try {
    const newActiveId = await invoke('close_tab', { tabId })
    state.tabs.delete(tabId)
    if (state.tabs.size === 0) { await createTab(); return }
    if (state.activeId === tabId && newActiveId) {
      state.activeId = newActiveId
      if (newActiveId) await invoke('switch_tab', { tabId: newActiveId })
    }
    renderTabs()
    updateNav()
  } catch (e) { console.error('close_tab failed', e) }
}

async function navigate(rawUrl) {
  if (!rawUrl?.trim()) return
  try {
    await invoke('navigate_tab', { tabId: state.activeId, url: rawUrl })
  } catch (e) { console.error('navigate_tab failed', e) }
}

// ── Bookmark / History panels ─────────────────────────────────────────────────
let historySearchTimer = null

async function openHistoryPanel() {
  closeAllPanels()
  const entries = await invoke('get_history', { limit: 50, offset: 0 })
  renderHistoryList(entries)
  $('historyPanel').classList.remove('hidden')
}

function renderHistoryList(entries) {
  $('historyList').innerHTML = entries.map(e => `
    <div class="panel-row" data-url="${escHtml(e.url)}">
      <div class="panel-row-favicon">${(e.url[8]||'?').toUpperCase()}</div>
      <div class="panel-row-text">
        <div class="panel-row-title">${escHtml(e.title)}</div>
        <div class="panel-row-url">${escHtml(e.url)}</div>
      </div>
    </div>
  `).join('')
  $('historyList').querySelectorAll('.panel-row').forEach(row => {
    row.addEventListener('click', () => { navigate(row.dataset.url); closeAllPanels() })
  })
}

async function openBookmarksPanel() {
  closeAllPanels()
  const bookmarks = await invoke('get_bookmarks')
  $('bookmarksList').innerHTML = bookmarks.map(b => `
    <div class="panel-row" data-url="${escHtml(b.url)}" data-id="${b.id}">
      <div class="panel-row-favicon">${(b.title[0]||'?').toUpperCase()}</div>
      <div class="panel-row-text">
        <div class="panel-row-title">${escHtml(b.title)}</div>
        <div class="panel-row-url">${escHtml(b.url)}</div>
      </div>
      <button class="panel-row-delete" data-bmid="${b.id}">✕</button>
    </div>
  `).join('')
  $('bookmarksList').querySelectorAll('.panel-row').forEach(row => {
    row.addEventListener('click', e => {
      if (e.target.closest('.panel-row-delete')) return
      navigate(row.dataset.url); closeAllPanels()
    })
  })
  $('bookmarksList').querySelectorAll('.panel-row-delete').forEach(btn => {
    btn.addEventListener('click', async e => {
      e.stopPropagation()
      await invoke('delete_bookmark', { id: btn.dataset.bmid })
      btn.closest('.panel-row').remove()
    })
  })
  $('bookmarksPanel').classList.remove('hidden')
}

function closeAllPanels() {
  ['menuDropdown','historyPanel','bookmarksPanel'].forEach(id => $
(id).classList.add('hidden'))
}

// ── Tauri events ──────────────────────────────────────────────────────────────
async function setupListeners() {
  await listen('tab-navigating', ({ payload }) => {
    const tab = state.tabs.get(payload.id)
    if (tab && payload.id === state.activeId) {
      $('addressInput').value = payload.url
    }
  })

  await listen('tab-loading', ({ payload }) => {
    const tab = state.tabs.get(payload.id)
    if (tab) {
      tab.loading = true
      tab.url = payload.url
      if (payload.id === state.activeId) {
        $('addressInput').value = payload.url
        $('newTabPage').classList.add('hidden')
      }
      renderTabs()
    }
  })

  await listen('tab-loaded', ({ payload }) => {
    state.tabs.set(payload.id, { ...state.tabs.get(payload.id), ...payload, loading: false })
    renderTabs()
    if (payload.id === state.activeId) updateNav()
  })
}

// ── Init ──────────────────────────────────────────────────────────────────────
async function init() {
  // Window controls
  $('btnClose')   ?.addEventListener('click', () => win.close())
  $('btnMinimize')?.addEventListener('click', () => win.minimize())
  $('btnMaximize')?.addEventListener('click', () => win.toggleMaximize())

  // Nav buttons
  $('btnBack')   .addEventListener('click', () => invoke('go_back',     { tabId: state.activeId }))
  $('btnForward').addEventListener('click', () => invoke('go_forward',  { tabId: state.activeId }))
  $('btnReload') .addEventListener('click', () => invoke('reload_tab',  { tabId: state.activeId }))
  $('btnNewTab') .addEventListener('click', () => createTab())

  // Bookmark
  $('btnBookmark').addEventListener('click', async () => {
    const tab = state.tabs.get(state.activeId)
    if (!tab?.url?.startsWith('http')) return
    const marked = await invoke('is_bookmarked', { url: tab.url })
    if (marked) {
      const bookmarks = await invoke('get_bookmarks')
      const bm = bookmarks.find(b => b.url === tab.url)
      if (bm) { await invoke('delete_bookmark', { id: bm.id }); $('btnBookmark').classList.remove('bookmarked') }
    } else {
      await invoke('add_bookmark', { url: tab.url, title: tab.title || tab.url })
      $('btnBookmark').classList.add('bookmarked')
    }
  })

  // Menu
  $('btnMenu').addEventListener('click', e => {
    e.stopPropagation()
    $('menuDropdown').classList.toggle('hidden')
  })
  $('menuHistory')  .addEventListener('click', () => openHistoryPanel())
  $('menuBookmarks').addEventListener('click', () => openBookmarksPanel())
  $('menuClearHistory').addEventListener('click', async () => {
    await invoke('clear_history'); closeAllPanels()
  })
  $('closeHistory')  .addEventListener('click', closeAllPanels)
  $('closeBookmarks').addEventListener('click', closeAllPanels)

  // History search (debounced 150ms)
  $('historySearch').addEventListener('input', e => {
    clearTimeout(historySearchTimer)
    historySearchTimer = setTimeout(async () => {
      const q = e.target.value.trim()
      const entries = q.length > 1
        ? await invoke('search_history', { query: q })
        : await invoke('get_history', { limit: 50, offset: 0 })
      renderHistoryList(entries)
    }, 150)
  })

  // Address bar
  $('addressInput').addEventListener('keydown', e => {
    if (e.key === 'Enter') { navigate(e.target.value); e.target.blur() }
    if (e.key === 'Escape') { updateNav(); e.target.blur() }
  })
  $('addressInput').addEventListener('focus', e => e.target.select())

  // Shortcuts on new-tab page
  document.querySelectorAll('.shortcut-btn').forEach(btn => {
    btn.addEventListener('click', () => navigate(btn.dataset.url))
  })

  // Keyboard shortcuts
  document.addEventListener('keydown', e => {
    const mod = e.metaKey || e.ctrlKey
    if (!mod) return
    switch (e.key) {
      case 't': e.preventDefault(); createTab(); break
      case 'w': e.preventDefault(); closeTab(state.activeId); break
      case 'l': e.preventDefault(); $('addressInput').focus(); break
      case 'r': e.preventDefault(); invoke('reload_tab', { tabId: state.activeId }); break
      case '[': e.preventDefault(); invoke('go_back',    { tabId: state.activeId }); break
      case ']': e.preventDefault(); invoke('go_forward', { tabId: state.activeId }); break
      default:
        if (e.key >= '1' && e.key <= '9') {
          e.preventDefault()
          const tabs = [...state.tabs.values()]
          const idx = parseInt(e.key) - 1
          if (tabs[idx]) switchTab(tabs[idx].id)
        }
    }
  })

  // Close dropdowns on outside click
  document.addEventListener('click', e => {
    if (!e.target.closest('.dropdown') && !e.target.closest('#btnMenu')) {
      closeAllPanels()
    }
  })

  // Setup Rust event listeners
  await setupListeners()

  // Load existing state (app re-activated etc.)
  const [existingTabs, activeId] = await Promise.all([
    invoke('get_tabs'),
    invoke('get_active_tab'),
  ])

  if (existingTabs.length > 0) {
    existingTabs.forEach(tab => state.tabs.set(tab.id, tab))
    state.activeId = activeId || existingTabs[0].id
  } else {
    await createTab()
    return
  }

  renderTabs()
  updateNav()
}

document.addEventListener('DOMContentLoaded', init)
```

**Step 2: Commit**

```bash
git add src/main.js
git commit -m "feat: complete frontend JS with tab management, history, bookmarks, keyboard shortcuts"
```

---

## Task 10: Build + Fix Compile Errors

**Files:** Any that fail to compile

**Step 1: Attempt dev build**

```bash
npm run tauri dev 2>&1 | tee /tmp/orbit-build.log
```

**Step 2: Fix common Tauri 2.10.x API issues**

If `app.get_webview(id)` doesn't exist, try `main_window.get_webview(id)`:
```rust
// Change:
app.get_webview(&tab_id)
// To:
app.get_webview_window("main")
    .and_then(|w| w.get_webview(&tab_id))
```

If `Webview::set_bounds` has different Rect signature:
```bash
# Check the actual Rect fields in Tauri 2.10.x:
grep -r "struct Rect" ~/.cargo/registry/src/*/tauri-2*/src/ 2>/dev/null | head -5
```

If `WebviewBuilder::visible(bool)` doesn't exist:
```rust
// After add_child returns the webview, set visibility:
let wv = main.add_child(builder, pos, size)?;
if !visible { wv.set_visible(false)?; }
```

If `Webview::navigate(url::Url)` returns `()` not `Result`:
```rust
// Remove the .map_err()
wv.navigate(parsed);
```

**Step 3: Run cargo tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -20
# Expected: all 9+ tests pass
```

**Step 4: Test app opens without crash**

The app should:
1. Open a window with dark chrome (108px)
2. Show the new-tab page with `nodaysidle` wordmark
3. Address bar focused on new tab
4. Typing a URL + Enter navigates to it in a native webview
5. Tab bar updates with title after page load

**Step 5: Commit**

```bash
git add -A
git commit -m "fix: resolve Tauri 2.10.x API compatibility issues"
```

---

## Task 11: Tauri Config + Bundle

**Files:**
- Modify: `src-tauri/tauri.conf.json`

**Step 1: Update tauri.conf.json**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Orbit",
  "version": "1.0.0",
  "identifier": "com.orbit.browser",
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devUrl": "http://localhost:1420",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "Orbit",
        "width": 1280,
        "height": 800,
        "minWidth": 600,
        "minHeight": 400,
        "center": true,
        "decorations": false,
        "transparent": false,
        "shadow": true,
        "backgroundColor": "#0c0c0e"
      }
    ],
    "security": {
      "csp": null,
      "dangerousDisableAssetCspModification": true
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "resources": ["resources/*"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "macOS": {
      "frameworks": [],
      "minimumSystemVersion": "10.15"
    }
  }
}
```

Note: `transparent: false` with solid `backgroundColor` avoids macOS compositing flicker that plagued the Electron prototype.

**Step 2: Update vite.config.js to use port 1420**

```javascript
import { defineConfig } from 'vite'

export default defineConfig({
  server: { port: 1420, strictPort: true },
  clearScreen: false,
})
```

**Step 3: Test full dev run**

```bash
npm run tauri dev
```

Verify:
- [ ] Window opens, dark background visible immediately (no flash)
- [ ] `nodaysidle` wordmark visible on new tab page
- [ ] All 4 shortcut icons visible with tooltips
- [ ] Typing URL navigates, tab title updates
- [ ] Back/forward buttons enable/disable correctly
- [ ] Cmd+T opens new tab, Cmd+W closes
- [ ] Login to Google, close app, reopen — still logged in

**Step 4: Build production app**

```bash
npm run tauri build 2>&1 | tail -30
# Creates: src-tauri/target/release/bundle/macos/Orbit.app
```

**Step 5: Install to /Applications**

```bash
cp -R src-tauri/target/release/bundle/macos/Orbit.app /Applications/Orbit.app
open /Applications/Orbit.app
```

**Step 6: Final commit**

```bash
git add src-tauri/tauri.conf.json vite.config.js
git commit -m "feat: production config, bundle resources, solid background"
```

---

## Compile Error Reference

Common Tauri 2.10.x issues and fixes:

| Error | Fix |
|-------|-----|
| `no method 'get_webview' on AppHandle` | Use `app.get_webview_window("main").and_then(\|w\| w.get_webview(id))` |
| `Rect` field `x/y` not found | Use `position: Position::Logical(...)` and `size: Size::Logical(...)` |
| `visible()` not on WebviewBuilder | Set after creation: `wv.set_visible(false)?` |
| `navigate()` type mismatch | Ensure you pass `url::Url` not `String` |
| `on_page_load` closure not Sync | Add `+ Sync` bound or use `Arc<Mutex<>>` for shared data |
| `optional()` not found | Import `rusqlite::OptionalExtension` |
