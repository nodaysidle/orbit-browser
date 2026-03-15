# Orbit Browser - Tauri 2 Implementation Plan

## Chosen Architecture
**Tauri 2 + Rust Backend** - Best for macOS native integration

## What I Attempted (Prototype Phase)
- Basic UI with glassmorphism design
- Tab management UI
- Navigation controls
- Attempted iframe approach (failed - security restrictions)

## Why iframe Failed
Modern sites (GitHub, etc.) use `X-Frame-Options` and CSP headers that block iframe embedding. **Must use native Webview API**.

## What Must Be Built (Tauri 2 Proper Implementation)

### Phase 1: Core Webview Management (8-10h)
```rust
// Rust backend commands needed:
- create_webview(tab_id, url) -> WebviewWindow
- navigate_webview(tab_id, url)
- destroy_webview(tab_id)
- show_webview(tab_id) // Hide others, show active
- get_webview_bounds() -> Position for UI overlay
```

**Implementation:**
- Create webviews as child windows positioned below UI chrome
- Handle tab switching via Rust state management
- Inject CSS to hide webview scrollbars/overlays where needed

### Phase 2: Navigation State (4-6h)
```rust
- get_nav_state(tab_id) -> { can_go_back, can_go_forward, is_loading }
- go_back(tab_id)
- go_forward(tab_id)
- reload(tab_id)
- stop_loading(tab_id)
```

**Implementation:**
- Use Tauri's webview event listeners
- Forward navigation events to frontend via emit/listen

### Phase 3: Data Persistence (6-8h)
```rust
// SQLite via sqlx or similar
- bookmarks: id, title, url, folder, created_at
- history: id, url, title, visit_count, last_visit
- settings: key, value
```

**Implementation:**
- Setup SQLite database in Rust
- Create commands: add_bookmark, get_bookmarks, add_history, search_history

### Phase 4: Ad Blocking (8-10h)
```rust
- intercept_request(tab_id, url) -> bool // allow/block
- load_blocklists() -> Vec<Regex>
```

**Implementation:**
- Use Tauri's custom protocol or request interception
- Block based on host + path patterns
- Support EasyList format

### Phase 5: UI Polish (4-6h)
- Smooth tab animations
- Loading indicators
- Error pages
- Downloads UI

## Total: ~30-40 hours

## Next Steps
1. I rebuild with proper Rust webview management
2. You test each phase as we go
3. We iterate on UI/UX

Ready to start with Phase 1?
