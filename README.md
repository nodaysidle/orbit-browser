# Orbit

**A native macOS browser — WKWebView tabs, local-first data, keyboard-first chrome.**

Orbit is a lightweight browser built with Tauri 2 and native WKWebView child webviews. It keeps the browser shell small, stores user data locally, and avoids Electron, telemetry, and account lock-in.

![macOS](https://img.shields.io/badge/platform-macOS-lightgrey?logo=apple)
![Tauri 2](https://img.shields.io/badge/Tauri-2.x-blue?logo=tauri)
![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)
![License](https://img.shields.io/badge/license-MIT-green)

## Features

- **Native rendering** — each tab runs in a native WKWebView child webview.
- **Local-first storage** — bookmarks, history, settings, session state, shortcuts, and zoom memory live in SQLite on this Mac.
- **macOS-native chrome** — restrained titlebar, tabs, address bar, focus rings, hover/active/disabled states, and light/dark themes.
- **Keyboard-first navigation** — tab creation, close, switch, address focus, reload, stop, back/forward, find, zoom, and reader mode shortcuts.
- **Reader Mode** — `Cmd+Shift+R` injects a local reading layout for supported pages.
- **Clean link copying** — the address copy button strips common tracking parameters such as `utm_*`, `fbclid`, and `gclid`.
- **Per-origin zoom memory** — zoom levels can persist per site.
- **Local domain blocking** — bundled domain and URL-pattern blocklist in `resources/adblock-patterns.json`.

## Requirements

- macOS 10.15+ (`minimumSystemVersion` in the Tauri bundle config)
- Node.js 20+
- Rust stable toolchain with Cargo on `PATH`

## Install from source

```bash
git clone https://github.com/nodaysidle/orbit-browser.git
cd orbit-browser
npm ci
npm run tauri -- build --bundles app
```

The built app is produced at:

```txt
src-tauri/target/release/bundle/macos/Orbit.app
```

Install it locally:

```bash
ditto src-tauri/target/release/bundle/macos/Orbit.app /Applications/Orbit.app
open -n /Applications/Orbit.app
```

## Development

Full Tauri app with native webviews:

```bash
npm ci
npm run tauri -- dev
```

Frontend-only Vite preview:

```bash
npm run dev
```

Frontend-only mode opens at `http://localhost:1420`. Browser IPC commands require the Tauri runtime.

## Testing

```bash
npm test
npm run build
npm run check:rust
npm run check
```

`npm run check` runs:

1. JavaScript unit tests via `node --test`
2. Production Vite build
3. Rust format check
4. Rust tests
5. Rust clippy with warnings denied

## macOS packaging

Build a `.app` bundle:

```bash
npm run tauri -- build --bundles app
```

Install the built app:

```bash
ditto src-tauri/target/release/bundle/macos/Orbit.app /Applications/Orbit.app
```

Verify the installed app:

```bash
test -d /Applications/Orbit.app
codesign --verify --deep --strict --verbose=2 /Applications/Orbit.app
open -n /Applications/Orbit.app
```

The current local bundle configuration uses ad-hoc signing (`signingIdentity = "-"`). Notarization is not performed by the local build command.

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| `Cmd+T` | New tab |
| `Cmd+W` | Close tab |
| `Cmd+L` | Focus address bar |
| `Cmd+R` | Reload |
| `Cmd+.` | Stop loading |
| `Cmd+[` / `Cmd+]` | Back / Forward |
| `Cmd+Shift+[` / `Cmd+Shift+]` | Previous / Next tab |
| `Cmd+1` … `Cmd+9` | Switch to tab by index |
| `Cmd+F` | Find in page |
| `Cmd+G` | Next find result |
| `Cmd+=` / `Cmd+-` / `Cmd+0` | Zoom in / out / reset |
| `Cmd+Shift+R` | Toggle Reader Mode |

## Architecture

| Layer | Stack |
|---|---|
| App shell | Tauri 2 |
| Browser engine | WKWebView child webviews |
| Backend | Rust 2021 |
| Frontend | Vanilla JavaScript + Vite |
| Storage | SQLite via `rusqlite` |

```txt
orbit-browser/
├── index.html
├── src/
│   ├── main.js             # State, commands, tab manager
│   ├── events.js           # DOM event bindings
│   ├── styles.css          # CSS entrypoint
│   ├── styles/             # Base, chrome, home, panels
│   └── utils/              # DOM, render, URL/theme helpers
├── src-tauri/
│   ├── src/main.rs         # App setup and command registration
│   ├── src/browser.rs      # Tab state and URL/history logic
│   ├── src/tabs.rs         # WKWebView lifecycle and navigation
│   ├── src/db.rs           # SQLite persistence
│   ├── src/adblock.rs      # Local blocking
│   ├── src/download.rs     # Download detection/storage
│   └── src/layout.rs       # Webview bounds/chrome contract
└── resources/adblock-patterns.json
```

## Data and privacy

- Application data: `~/Library/Application Support/com.orbit.browser/orbit.db`
- No built-in analytics or telemetry.
- Search queries use the selected search engine only when address input is not a URL.
- The domain blocklist is bundled locally; Orbit does not fetch a remote list at runtime.

## License

MIT
