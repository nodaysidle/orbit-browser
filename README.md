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

## Premium Stress-Test Build – June 2026

This build is the current Orbit source and package line for NDI stress testing. The older GitHub package is superseded; release artifacts now target `v1.0.2`. Public notarization is intentionally deferred until after NDI manual stress testing.

### What improved in this pass

- **Dark-by-default fixed** — first-run theme now matches the product claim instead of falling back to a mismatched saved/default state.
- **Reader Mode upgraded** — replaced the weak CSS-only restyle with a reversible local extraction overlay that targets `article`, `main`, `[role="main"]`, or body fallback and strips noisy page chrome.
- **Runtime tab reorder proof** — added keyboard active-tab reorder (`Cmd+Opt+Shift+←/→`) and runtime smoke evidence that persisted tab order survives through SQLite session state.
- **Visual QA added** — new deterministic Playwright QA captures dark/light screenshots, checks overflow, proves visible keyboard focus, and records frame timing.
- **Smoke coverage expanded** — built-app smoke now covers real navigation, domain blocking, session order, find, reader mode, zoom/reset, reload/stop, and download cancel/no-file behavior.
- **Docs corrected** — test count and release notes now match the current source gates.

### Current verified quality gates

- `npm test`
- `npm run check`
- `scripts/premium-visual-qa.sh`
- `npm run tauri -- build --bundles dmg`
- `codesign --verify --deep --strict --verbose=2 src-tauri/target/release/bundle/macos/Orbit.app`
- `scripts/smoke-runtime.sh` with `ORBIT_APP_PATH` pointed at the built app

---

## Major Update – May 2026: Comprehensive UI/UX Overhaul

This release represents a major, design-driven improvement to Orbit's daily usability while strictly maintaining the project's core constraints and identity (Vanilla JavaScript only, no new Rust dependencies, preserved frontend module structure, WKWebView child webviews, locked-down CSP, and the distinctive warm amber glassmorphism aesthetic).

### Completed Work

- Full independent code audit + detailed design document (produced via structured review process and fully approved)
- Implementation of the complete approved 7-slice UI/UX polish plan, including:
  - Tab bar overflow affordances with elegant edge gradient masks and dynamic indicators
  - Address bar enhancements (persistent security pill, one-click copy button, click-to-copy on preview tooltip)
  - New-tab page elevation (subtle breathing animation on the orbiting rings logo, richer empty states with quick suggestions, inline shortcut deletion)
  - Full light theme visual parity pass (native macOS feel rather than inverted dark)
  - Accessibility, micro-interactions, keyboard discoverability, and motion polish
  - Persisted drag-to-reorder plus keyboard tab reorder for accessible runtime QA
- Five additional high-value features:
  1. **Per-origin zoom memory** — Zoom levels now persist per site
  2. **Smart clean link copying** — Automatically strips common tracking parameters (`utm_*`, `fbclid`, `gclid`, etc.)
  3. **Local-only Reader Mode** — Toggle with `Cmd+Shift+R` for a clean, comfortable reading experience
  4. **Improved find-in-page** — Better feedback and structure
  5. **Tab hibernation foundation** + supporting infrastructure (`eval_on_tab` command)
- Multiple verified clean production builds with the app installed to `/Applications/Orbit.app` (previous versions removed before each final install)

All changes were developed with repeated `npm run check` validation (31 JS tests + 69 Rust tests + clippy + production Vite build) and respect the 9.7/10 quality bar.

### New / Enhanced Keyboard Shortcuts

| Key            | Action                    |
|----------------|---------------------------|
| `Cmd+Shift+R`  | Toggle Reader Mode        |
| `Cmd+=` / `-`  | Zoom in / out (now persists per origin) |
| `Cmd+Opt+Shift+←/→` | Move active tab left / right |
| Copy button    | Copies clean link (tracking stripped) |

---

## What this app is

Orbit is a native Tauri 2.x + Rust macOS browser that uses WKWebView-backed child webviews for each tab. It provides lightweight browser chrome (tabs, address bar, history, bookmarks, startup state), local persistence with rusqlite, and a privacy-forward "local-first" behavior with no built-in telemetry.

## Latest Major Update (2026-05-30)

See the dedicated section above ("Major Update – May 2026") for the full list of completed work, including the comprehensive UI/UX overhaul and five new features.

Previous session fixes (2026-05-28) remain relevant:
- Fixed session restore startup behavior so restored tabs no longer trigger an extra navigation on launch.
- Reworked startup restore flow to call tab switching from Rust state instead of reloading the same URL through the address path.

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

### Download

- GitHub release: https://github.com/nodaysidle/orbit-browser/releases/tag/v1.0.2
- Direct DMG: https://github.com/nodaysidle/orbit-browser/releases/download/v1.0.2/Orbit-1.0.2-aarch64.dmg

The release build is ad-hoc signed and locally verified for local/internal use. It is not Apple-notarized yet, so external distribution still needs Developer ID signing plus notarization.


### Requirements

- macOS 14+
- Node.js 20+
- Rust stable toolchain (`rustup`)
- `cargo` available on `PATH` (for rustup installs that is usually `export PATH="$HOME/.cargo/bin:$PATH"`)

### From source

```bash
git clone https://github.com/nodaysidle/orbit-browser.git
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
| `Cmd+Shift+R` | Toggle Reader Mode |

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

Built by **[NODAYSIDLE](https://github.com/nodaysidle)** — 9.7/10 bar, every time.

---

*License: MIT*
