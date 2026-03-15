<div align="center">

# Orbit

**A minimal, fast browser built with Tauri 2 + Rust.**
Native webviews. No Electron. No telemetry. No noise.

[![Platform](https://img.shields.io/badge/platform-macOS-333?style=flat-square&logo=apple)](https://gitlab.com/NODAYSIDLE/orbit-browser)
[![Tauri](https://img.shields.io/badge/tauri-2-24c8db?style=flat-square&logo=tauri)](https://tauri.app)
[![Rust](https://img.shields.io/badge/rust-1.75-f74c00?style=flat-square&logo=rust)](https://rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-d4a72c?style=flat-square)](./LICENSE)

</div>

---

## Why

Most browsers are bloated. Orbit is not.

- **Native WKWebView per tab** — no iframe restrictions, no X-Frame-Options blocks, full site compatibility
- **Sessions persist** — log into Google once, it stays logged in (WKWebView persistent store)
- **No telemetry** — not even Google Fonts on first load (all fonts are self-hosted)
- **Ad blocking** — domain matching in Rust before any network request leaves the machine
- **Your data stays local** — bookmarks and history in a SQLite file on your machine, nowhere else

---

## Features

| | |
|---|---|
| **Native tabs** | One WKWebView child per tab — any site, no restrictions |
| **Ad blocking** | O(1) domain + subdomain matching in Rust via `on_navigation` hook |
| **Bookmarks** | Save, browse, and delete from the nav bar |
| **History** | Full browsing history with live debounced search |
| **Session persistence** | Cookies, localStorage, IndexedDB survive restarts automatically |
| **New tab page** | `nodaysidle` wordmark + 4 shortcuts with hover tooltips |
| **Self-hosted fonts** | Space Grotesk (UI) + JetBrains Mono (wordmark) — zero CDN requests |
| **Keyboard shortcuts** | Full set — see table below |

---

## Install

### macOS — from source

```bash
git clone https://gitlab.com/NODAYSIDLE/orbit-browser.git
cd orbit-browser
npm install
npm run tauri build
cp -R src-tauri/target/release/bundle/macos/Orbit.app /Applications/
```

### Dev mode

```bash
npm install
npm run tauri dev
```

---

## Keyboard shortcuts

| Shortcut | Action |
|:---------|:-------|
| `⌘T` | New tab |
| `⌘W` | Close tab |
| `⌘L` | Focus address bar |
| `⌘R` | Reload |
| `⌘[` | Go back |
| `⌘]` | Go forward |
| `⌘1`–`⌘9` | Switch to tab N |

---

## Architecture

```
orbit-browser/
├── src-tauri/src/
│   ├── main.rs        — app setup, all Tauri commands, webview lifecycle
│   ├── browser.rs     — TabInfo, TabData, BrowserState, URL normalization
│   ├── db.rs          — SQLite schema, bookmarks, history, settings
│   └── adblock.rs     — domain blocklist loader, O(1) HashSet matching
│
├── src/
│   ├── index.html     — chrome shell (titlebar, tab bar, nav bar, new tab page)
│   ├── styles.css     — full design system (graphite palette, Space Grotesk)
│   ├── main.js        — frontend logic (invoke/listen via @tauri-apps/api)
│   └── fonts/         — SpaceGrotesk + JetBrainsMono woff2 (self-hosted)
│
└── resources/
    └── adblock-patterns.json   — blocked domains list
```

### How tabs work

Each tab is a native `Webview` child added to the main window via `Window::add_child()` at `y=108px` (below the 108px chrome). Only the active tab is visible — others are hidden with `wv.hide()`. On window resize, the active webview bounds are updated in real time.

New blank tabs have no webview until the first navigation (lazy creation).

### How ad blocking works

On each navigation request, `WebviewBuilder::on_navigation` fires in Rust before any network call. The blocked domain set (`Arc<HashSet<String>>`) is checked for exact match and subdomain suffix match. Returning `false` drops the request at the OS level — nothing leaves the machine.

---

## Privacy

| What | Where |
|:-----|:------|
| Bookmarks | `~/Library/Application Support/com.orbit.browser/orbit.db` |
| History | Same SQLite file |
| Fonts | Bundled in the app — zero network requests for typography |
| Search queries | Sent to DuckDuckGo (their privacy policy applies) |
| Telemetry | None |
| Analytics | None |

---

## Stack

| | |
|---|---|
| **Backend** | Rust + Tauri 2 |
| **Webviews** | WKWebView (native macOS) via Tauri unstable child webview API |
| **Database** | rusqlite (bundled SQLite, no system dependency) |
| **Frontend** | Vanilla JS + `@tauri-apps/api` |
| **Fonts** | Space Grotesk (UI), JetBrains Mono (wordmark) |
| **Build** | Vite (frontend) + cargo (Rust) |

---

## Customise ad block rules

Edit `resources/adblock-patterns.json`:

```json
{
  "domains": ["example-tracker.com", "ads.example.net"]
}
```

Rebuild to apply changes. The file is bundled into the `.app` at build time.

---

<div align="center">
  <sub>Built by <a href="https://gitlab.com/NODAYSIDLE">NODAYSIDLE</a></sub>
</div>
