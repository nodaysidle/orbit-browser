# Orbit

**A focused macOS browser — native tabs, local-first, dark by default.**

Minimal chrome, full web. WKWebView child webviews on Tauri 2. No Electron, no telemetry, no noise.

<p>
  <img src="https://img.shields.io/badge/platform-macOS-black?style=flat-square&logo=apple" alt="macOS">
  <img src="https://img.shields.io/badge/tauri-2.10-f0b35f?style=flat-square&logo=tauri" alt="Tauri 2.10">
  <img src="https://img.shields.io/badge/rust-stable-8b7355?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/license-MIT-7b8cac?style=flat-square" alt="MIT">
</p>

---

## About

Orbit is a native macOS browser built for people who want the web without the overhead. No Chrome bloat, no Electron memory tax, no analytics pinging home.

| Layer | Stack |
|-------|-------|
| Shell | Tauri 2.10 |
| Engine | WKWebView (native child webviews) |
| Backend | Rust 2021 |
| Frontend | Vanilla JS + Vite |
| Storage | SQLite via rusqlite |

---

## Features

- **Native tabs** — each tab gets its own WKWebView, managed by Rust. No multi-process overhead.
- **Local-first data** — bookmarks, history, settings stay in a bundled SQLite DB. Your data, your machine.
- **Domain blocking** — built-in blocklist at `resources/adblock-patterns.json`
- **Keyboard-first** — full shortcut set for tabs, navigation, find-in-page
- **Dark by default** — amber-accented dark theme, with light mode as secondary option
- **Locked-down CSP** — script-src is tight in Tauri config

---

## Install

### Requirements

- macOS 14+
- Node.js 20+
- Rust stable toolchain (`rustup`)

### From source

```bash
git clone git@gitlab.com:NODAYSIDLE/orbit-browser.git
cd orbit-browser
npm ci
npm run tauri build
```

The `.app` lands at `src-tauri/target/release/bundle/macos/Orbit.app`.

### Quick install

```bash
npm run tauri build && \
ditto src-tauri/target/release/bundle/macos/Orbit.app /Applications/Orbit.app
```

Or use the helper:

```bash
bash scripts/build-mac.sh   # builds the app
bash scripts/install-app.sh # copies to /Applications
```

---

## Development

```bash
npm ci
npm run tauri dev
```

Frontend-only hot-reload:

```bash
npm run dev
```

Opens at `http://localhost:1420` — API calls won't work outside Tauri.

---

## Quality

```bash
npm run check
```

Runs:

1. JS unit tests (`node --test`)
2. Production Vite build
3. Rust format check, clippy, and tests

---

## Shortcuts

| Key | Action |
|-----|--------|
| `Cmd+T` | New tab |
| `Cmd+W` | Close tab |
| `Cmd+L` | Focus address bar |
| `Cmd+R` | Reload |
| `Cmd+.` | Stop |
| `Cmd+[` / `Cmd+]` | Back / Forward |
| `Cmd+Shift+[` / `Cmd+Shift+]` | Previous / Next tab |
| `Cmd+1` … `Cmd+9` | Switch to tab by index |
| `Cmd+F` | Find in page |

---

## Project

```
orbit-browser/
├── index.html              # Browser chrome layout
├── src/
│   ├── main.js             # State, commands, tab manager
│   ├── events.js           # DOM event bindings
│   ├── styles.css          # CSS entrypoint
│   ├── styles/
│   │   ├── base.css        # Variables, reset, global
│   │   ├── chrome.css      # Titlebar, tabs, nav bar
│   │   ├── home.css        # New-tab page
│   │   └── panels.css      # Dropdowns, history, bookmarks
│   └── utils/
│       ├── ui.js           # Theme, URL helpers
│       ├── render.js       # DOM rendering
│       └── dom.js          # Element/icon factory
├── src-tauri/
│   ├── src/
│   │   ├── main.rs         # App setup, commands
│   │   ├── browser.rs      # Tab state, navigation
│   │   ├── tabs.rs         # Webview lifecycle
│   │   ├── db.rs           # SQLite persistence
│   │   ├── adblock.rs      # Domain blocking
│   │   ├── download.rs     # File downloads
│   │   └── layout.rs       # Webview positioning
│   └── tauri.conf.json
└── resources/
    └── adblock-patterns.json
```

---

## Data & Privacy

- **Database:** `~/Library/Application Support/com.orbit.browser/orbit.db`
- **No telemetry.** No analytics. No background pings.
- **Search:** unrecognized input sends queries to DuckDuckGo. No logging.
- **Blocklist:** local file, no remote fetch.

---

## Brand

App icon and assets under `src-tauri/icons/`. Source logo at `build/icons/orbit-logo.svg`.

Built by **[NODAYSIDLE](https://gitlab.com/NODAYSIDLE)** — 9.7/10 bar, every time.

---

*License: MIT*
