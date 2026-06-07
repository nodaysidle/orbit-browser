## Stack
Tauri 2.x — Rust backend + Vanilla JS frontend (macOS browser)

## Identity
NODAYSIDLE. Quality bar: 9.7/10. Ship installable, polished apps.
Team: NDI (code) + Kaly (design/UI/QA).

## Global Rules
Apply all NODAYSIDLE global rules (brand, code standards, audit format, UI/UX).
If existing code doesn't match global rules: this is a refactoring task — tell the AI explicitly.

## Subagent Override
multi_agent_v2 is DISABLED for this project. Do not use spawn_agent, send_input, resume_agent, wait_agent, or close_agent. Work sequentially in a single session with no delegation. This overrides the Subagent Dispatch section in ~/.codex/AGENTS.md.

## Project-Specific Constraints
- Browser engine: WKWebView on macOS via Tauri child webviews (unstable feature)
- Frontend: Vanilla JavaScript. No React/Vue/Svelte. No framework.
- Backend: Rust with rusqlite (bundled SQLite), no ORM
- Build: npm run tauri -- build (or cargo tauri build). Releases go to src-tauri/target/release/bundle/macos/Orbit.app
- Verify before claiming done: app launches, dark mode renders, tabs work, bookmarks/history persist
- CSP is locked down in tauri.conf.json — do not relax script-src
- Config: src-tauri/tauri.conf.json, src-tauri/Cargo.toml, package.json

## Do Not
- Add JavaScript frameworks (no React, Vue, Svelte, jQuery, Alpine)
- Add Electron or switch from WKWebView
- Remove or relax CSP directives
- Change the frontend module structure (src/main.js → events.js → utils/render.js, utils/ui.js, utils/dom.js)
- Add new Rust dependencies without asking
- Remove unstable feature from Cargo.toml (required for child webviews)
- Touch src-tauri/build.rs

## File Layout
/
  index.html              — browser chrome, tab bar, nav bar, new-tab page, panels
  src/
    main.js               — state, commands, tab management, keyboard shortcuts
    events.js             — DOM event binding
    styles.css            — CSS entrypoint (imports base, chrome, home, panels)
    styles/
      base.css            — CSS variables, fonts, reset, global styles
      chrome.css          — titlebar, tab bar, nav bar, address bar, buttons
      home.css            — new-tab page, logo, shortcuts
      panels.css          — dropdowns, history/bookmark panels, toasts, find bar
    utils/
      ui.js               — URL normalization helpers, theme, navigation snapshots
      render.js           — DOM rendering for tabs, history, bookmarks lists
      dom.js              — element factory, icon factory
    fonts/
      JetBrainsMono-Regular.woff2
      InstrumentSerif-Regular.ttf
  src-tauri/
    src/
      main.rs             — app setup, DB init, menu, session restore, command registration
      browser.rs          — TabInfo, TabData, BrowserState, URL normalization, history logic
      tabs.rs             — create_tab, switch_tab, close_tab, navigate_tab, go_back/forward, reload, stop, zoom, find, webview management
      db.rs               — SQLite: bookmarks, history, settings, session persistence
      adblock.rs          — domain blocklist loader and URL matcher
      download.rs         — download URL detection and file download
      layout.rs           — webview positioning, bounds calculation
    tauri.conf.json        — window config, CSP, bundle icons, macOS settings
    Cargo.toml
  resources/
    adblock-patterns.json  — blocked domains list
  package.json
  vite.config.ts

## How Tabs Work
Blank tabs start as state-only (no webview). On first navigation, Rust creates
a native child WKWebView below the 124px browser chrome. Active webview is shown
and resized with window; inactive tabs are hidden. Uses Tauri unstable feature
for child webview APIs.

## How Domain Blocking Works
on_navigation callback checks top-level URLs against adblock-patterns.json before
allowing navigation. Blocks exact domains, subdomain suffixes, and literal patterns.
