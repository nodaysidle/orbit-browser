# Changelog

## 2026-06-16 — Native macOS redesign PR

- Refreshed Orbit's public GitHub README for the native macOS redesign branch, with clearer positioning around WKWebView child webviews, local-first SQLite data, vanilla JavaScript, and Tauri DMG packaging.
- Documented the local DMG build path used by the release-polish gate: `src-tauri/target/release/bundle/dmg/`.
- Preserved the existing release boundary: this branch prepares PR documentation and local packaging proof only; publishing a GitHub Release remains a separate approval gate.

## 2026-05-30 — Major UI/UX Overhaul & Feature Additions

**This was a large, design-driven release.** A full independent code audit was performed, resulting in a detailed design document that was reviewed and fully approved. The entire approved plan was then executed, followed by five additional high-value features.

### Key Outcomes
- Comprehensive visual and interaction polish across the entire browser chrome while preserving the distinctive warm amber glassmorphism aesthetic and "Minimal chrome, full web" identity.
- Multiple clean production builds with strict discipline: previous `/Applications/Orbit.app` was always removed before installing the newly built version.
- All work strictly followed project constraints (Vanilla JavaScript only, exact frontend module structure preserved, no new Rust dependencies, WKWebView child webviews, locked CSP, no modifications to `build.rs`).

### Implemented Work

**Major UI/UX Polish Wave (7 slices from approved design document):**
- Tab bar elevation: Edge gradient masks for overflow + dynamic left/right indicators.
- Address bar improvements: Persistent security pill (HTTPS/HTTP), dedicated copy button, click-to-copy on the URL preview tooltip.
- New-tab page elevation: Subtle breathing animation on the orbiting rings logo + core, richer empty states with quick suggestion pills, inline shortcut deletion on the new-tab surface.
- Full light theme visual parity (component-level overrides so light mode feels native rather than inverted dark).
- Accessibility, micro-interactions, discoverability (tooltips on nav buttons, better ARIA, motion refinements).
- Final end-to-end visual QA, motion polish, and verification gate.
- Frontend foundation for tab drag-to-reorder (visual + interaction only; persistence intentionally left as optional pending explicit approval for any Rust surface).

**Five Additional Features (post-approval recommendations):**
- Per-origin zoom memory (zoom levels now persist per site).
- Smart clean link copying — the copy button and preview tooltip now automatically strip common tracking parameters (`utm_*`, `fbclid`, `gclid`, `ref`, etc.).
- Local-only Reader Mode (toggle with `Cmd+Shift+R`) — applies a comfortable, serif-based reading stylesheet directly in the page.
- Improved find-in-page experience and structure.
- Tab hibernation foundation + supporting infrastructure.

**Technical Additions:**
- New internal `eval_on_tab` Tauri command to safely execute JavaScript on specific child webviews (enables Reader Mode, zoom application, and future local enhancements).

**Process & Quality:**
- Full `npm run check` validation (25 JS tests + 60 Rust tests + format + clippy `-D warnings` + production Vite build) after every significant change batch.
- The complete approved design document is archived at `docs/design/2026-05-29-orbit-ui-ux-polish-approved.md`.

## 2026-05-25

- Added a polished dark-first new-tab experience with a centered search bar, recent pages, editable shortcuts, in-page navigation errors, full-chrome loading feedback, and a native-feeling settings panel.
- Hardened production error visibility, late tab callback handling, history persistence, and expanded JS/Rust regression coverage.
- Made Orbit's custom chrome feel more macOS-native with Safari-style traffic light hover symbols, system-aware themes, a fuller macOS menu bar, tighter tabs, system UI fonts, native-feeling scrollbars/tooltips, and a compact About Orbit panel.
- Added native Back, Forward, and Find menu accelerators so page navigation shortcuts work while a WKWebView has keyboard focus.
- Kept find-in-page keyboard focus anchored after match navigation so Escape reliably closes the find bar.

## 2026-05-23 (v1.3.1 — AeroSpace fix)

- **AeroSpace tiling fix:** Webview creation now queries the live window size directly instead of using cached `window_size`, which could be stale when AeroSpace tiles the window between HTML load and first navigation. Also removed `center: true` (fights tiling WMs) and added `WindowEvent::Moved` to the resize handler so AeroSpace repositioning triggers re-layout.
- Removed dead `active_webview_position_and_size` function. Build: 0 warnings.

## 2026-05-23 (v1.3)

- **Favicon rendering:** Tab bar now shows actual favicon images instead of monogram letters. Falls back through chain: standard `/favicon.ico` → Google's `s2/favicons` service → monogram letter. CSS for `.tab-favicon` and `.tab-icon-wrapper` sizes.
- **Expanded tests:** +10 new tests (favicon URL construction, download edge cases: .csv, .json, .deb, .php, filename extraction). Total test suite: **55 tests** (46 Rust + 9 JS). All passing.
- **Build:** 13MB release binary, signed, DMG-ready.

## 2026-05-23 (v1.2)

- **Downloads:** Full download support added. File extensions (.zip, .pdf, .dmg, .mp4, etc.) detected at navigation level. Files downloaded via reqwest to ~/Downloads. Toast notifications for start/complete. Navigation blocked for download URLs with `download-detected` event.
- **Favicons:** Auto-fetched on page load via `favicon_from_url()`. Emitted as `tab-favicon` event to frontend for display.
- **Tab cycling:** Cmd+Shift+[ and Cmd+Shift+] cycle through tabs.
- **ESC handling:** ESC closes find bar, then panels.
- **10 new download tests.** Total test suite: 45 tests (36 Rust + 9 JS).
- Added `reqwest` with rustls-tls to dependencies.

## 2026-05-23 (v1.1)

- **Retina display fix (complete):** `fallback_logical_size()` now divides physical pixels by scale factor. Window resize handler pre-populates `window_size` with correct logical dimensions before layout calculation. Fixes the race condition where the fallback path returned 2× coordinates on Retina displays.
- **Find in Page (Cmd+F):** Added find bar with next/prev navigation. WKWebView's native `window.find()` integration for highlighting. Cmd+G for find next.
- **Zoom controls:** Cmd+= / Cmd+- to zoom pages in/out. Cmd+0 to reset zoom.
- **Session persistence:** Tabs are now saved to SQLite on every tab create/close/navigate. Restored automatically on app launch. No more blank slate on restart.
- **Tab state save:** `save_session` command and automatic Rust-side persistence on all tab mutations.

## 2026-05-23

- Fixed blocked-navigation rollback so refused pages no longer leave tabs in a fake loading state.
- Added a persisted dark/light mode toolbar toggle with dark mode as the default.
- Enforced bookmark URL uniqueness in SQLite and made bookmark saves atomic.
- Enabled literal URL-pattern blocking from the bundled blocklist.
- Hardened async history search, script CSP, and installer rollback behavior.
- Rebuilt the Orbit browser chrome with a lighter responsive theme, safer list rendering, and no visible placeholder copy.
- Hardened tab navigation, history state, bookmark deduplication, domain blocking, and SQLite query bounds.
- Replaced stale Electron build/install scripts with Tauri macOS scripts and refreshed project documentation.
- Added the Tauri main-window capability so startup IPC, tabs, bookmarks, history, and window controls are authorized.
- Limited the default macOS bundle target to the signed app bundle used by the installer.
- Fixed blank-tab activation so the previous page is hidden and shortcuts navigate the active new tab directly.
- Added native macOS menu accelerators for tab, address, and reload actions so shortcuts work while a page has focus.
- Authorized native titlebar dragging and removed an unused shell plugin to prevent startup/window-chrome ACL errors.
- Fixed native page sizing on Retina displays so websites fill the browser window instead of rendering as a half-height view.
- Synced native page bounds from the browser chrome on startup, navigation, tab switches, and window resizes for tiled-window workflows.
