# Orbit Browser — Design Document
*2026-03-15*

## Overview

A fast, beautiful, minimal browser built with Tauri 2 + Rust. Single main window with native child webviews per tab. Designed for AeroSpace tiling on a 32" monitor — chrome is compact, content-first.

---

## Architecture

### Window Layout

```
┌─────────────────────────────────────────────────────────┐
│  Main Window (decorations: false, transparent: true)    │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Chrome UI Webview (tauri://localhost)  108px   │    │
│  │  ├── Titlebar (32px) — traffic lights, drag     │    │
│  │  ├── Tab Bar  (36px) — tabs + new tab button    │    │
│  │  └── Nav Bar  (40px) — back/fwd/reload/address  │    │
│  └─────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Tab Webview (child, y=108, external URLs)      │    │
│  │  • One per tab, only active one is visible      │    │
│  │  • Native WKWebView — bypasses X-Frame-Options  │    │
│  │  • on_page_load → emits events to chrome UI     │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### Data Flow

- Chrome JS → `invoke('create_tab', ...)` → Rust creates child webview
- Page loads → `on_page_load` callback → Rust emits `tab-loaded` → Chrome JS updates UI
- User navigates → `invoke('navigate_tab')` → `webview.navigate(url)`
- Back/forward → `invoke('go_back')` → `webview.eval("history.back()")`
- SQLite entirely in Rust; JS calls commands (`add_bookmark`, `search_history`, etc.)
- Ad blocker intercepts requests in Rust before network via `on_navigation` return false

### Session Persistence

WKWebView default persistent data store: cookies, localStorage, IndexedDB survive restarts automatically. Google login, GitHub, all sessions persist. No configuration needed.

---

## Rust Backend

### State

```rust
BrowserState {
    tabs: Mutex<HashMap<String, TabData>>,
    active_tab: Mutex<Option<String>>,
}

TabData {
    info: TabInfo,        // id, url, title, loading, can_go_back, can_go_forward
    history: Vec<String>, // URL history stack
    history_idx: usize,
    has_webview: bool,    // false for new tabs not yet navigated
}
```

### Commands

**Tab management:**
- `create_tab(url, make_active)` → TabInfo
- `switch_tab(tab_id)` → ()
- `close_tab(tab_id)` → Option<String> (new active id)
- `navigate_tab(tab_id, url)` → ()
- `go_back(tab_id)` → ()
- `go_forward(tab_id)` → ()
- `reload_tab(tab_id)` → ()
- `get_tabs()` → Vec<TabInfo>
- `get_active_tab()` → Option<String>

**Bookmarks:**
- `add_bookmark(url, title)` → ()
- `get_bookmarks()` → Vec<Bookmark>
- `delete_bookmark(id)` → ()
- `is_bookmarked(url)` → bool

**History:**
- `add_history(url, title)` → ()
- `get_history(limit, offset)` → Vec<HistoryEntry>
- `search_history(query)` → Vec<HistoryEntry>
- `clear_history()` → ()

**Settings:**
- `get_setting(key)` → Option<String>
- `set_setting(key, value)` → ()

### Events (Rust → Frontend)

- `tab-loading` — `{ id, url }`
- `tab-loaded` — `{ id, url, title, can_go_back, can_go_forward }`

### Ad Blocker

- Load EasyList-format blocklist at startup into a `HashSet<String>` (domain-based)
- Register via `on_navigation` on each child webview — return `false` to block
- Blocked domains: ads, trackers, analytics (bundled default list ~50k entries)
- O(1) lookup per navigation

### URL Normalization

- `https?://` → pass through
- `word.tld` (no spaces, has dot) → prepend `https://`
- Anything else → `https://duckduckgo.com/?q=<encoded>`

### History Deduplication

On `add_history`: UPDATE visit_count+1 + last_visited if URL exists, else INSERT.

---

## Database Schema (SQLite via rusqlite bundled)

```sql
CREATE TABLE bookmarks (
    id          TEXT PRIMARY KEY,
    url         TEXT NOT NULL,
    title       TEXT NOT NULL,
    favicon     TEXT,
    created_at  INTEGER NOT NULL
);

CREATE TABLE history (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    url          TEXT NOT NULL UNIQUE,
    title        TEXT NOT NULL,
    favicon      TEXT,
    visit_count  INTEGER DEFAULT 1,
    last_visited INTEGER NOT NULL
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE INDEX idx_history_last_visited ON history(last_visited DESC);
CREATE INDEX idx_history_url ON history(url);
```

---

## Frontend Design

### Visual Language

**Palette:**
```
Background:     #0c0c0e
Surface:        #131316
Surface-raised: #1a1a1f
Border:         rgba(255,255,255,0.07)
Text:           #f0f0f2
Text-dim:       rgba(240,240,242,0.45)
Accent:         #4f8ef7
Accent-glow:    rgba(79,142,247,0.15)
Danger:         #f75f4f
```

**Typography:**
- `Space Grotesk` — all UI chrome (self-hosted)
- `JetBrains Mono` — `nodaysidle` wordmark on new tab page (self-hosted)

### Chrome Components

**Titlebar (32px):** Traffic lights left (CSS circles, functional), drag region, tab count indicator right.

**Tab Bar (36px):** Tabs min-width 100px, max-width 200px, overflow scroll hidden. Active tab: 2px left blue border + surface-raised bg. Loading: CSS sweep animation underline. Favicon 14px or letter monogram fallback.

**Nav Bar (40px):** Icon buttons (back/fwd/reload), address bar (command-line style — borderless rest, blue shadow focus), bookmark toggle, menu.

### New Tab Page

```
          (centered vertically)

      nodaysidle
      ──────────   (48px wide, 1px, accent)

    [GL]  [YT]  [PH]  [TG]
```

- `nodaysidle`: JetBrains Mono 52px 600 weight, letter-spacing -0.02em, color #f0f0f2
- Separator: accent color `#4f8ef7`
- Shortcuts: 44px circles, SVG icon inside (monochrome, text-dim at rest)
  - Hover: icon full white + `box-shadow: 0 0 12px rgba(79,142,247,0.35)`
  - Tooltip: Space Grotesk 11px, appears 6px below on hover

**Shortcut targets:**
- GL → `https://gitlab.com/NODAYSIDLE` (GitLab fox SVG)
- YT → `https://youtube.com` (YouTube play button SVG)
- PH → `https://producthunt.com` (ProductHunt logo SVG)
- TG → `https://telegram.org` (Telegram paper plane SVG)

### Address Bar Behavior

- Focus: select all text
- Enter: normalize URL and navigate
- `Escape`: blur, restore original URL
- Shows loading spinner (CSS) while page loading

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+T` | New tab |
| `Cmd+W` | Close tab |
| `Cmd+L` | Focus address bar |
| `Cmd+R` | Reload |
| `Cmd+[` | Back |
| `Cmd+]` | Forward |
| `Cmd+1-9` | Switch to tab N |
| `Cmd+D` | Toggle bookmark |

---

## Performance

- Child webviews created lazily — new blank tab has no webview until first navigation
- Ad blocker `HashSet` loaded once at startup, never rebuilt
- History search: 150ms JS debounce before `invoke`
- All animations: CSS only (no JS animation loops)
- Fonts self-hosted (no network request for typography)
- Tab favicon: CSS letter monogram fallback (no broken image flicker)

---

## File Structure

```
src-tauri/src/
├── main.rs          — app setup, window events, command registration
├── browser.rs       — tab state, webview lifecycle, navigation
├── db.rs            — SQLite schema, bookmark/history commands
└── adblock.rs       — blocklist loading, domain matching

src/
├── index.html       — chrome shell (no iframe)
├── styles.css       — all visual styles
├── main.js          — browser logic, invoke/listen
└── fonts/           — Space Grotesk, JetBrains Mono woff2 files
```
