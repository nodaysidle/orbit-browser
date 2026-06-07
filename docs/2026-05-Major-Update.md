# Major Update – May 2026: Technical Summary

**Date:** 2026-05-30  
**Scope:** Comprehensive UI/UX overhaul + five additional features  
**Process:** Full code audit → detailed design document → approved plan execution → additional features → multiple clean production builds & installs

---

## 1. Background & Process

A complete, independent code review and audit of the Orbit codebase was performed. This led to the creation of a structured design document that was subjected to a formal write → review → revise loop until it received zero open issues from the reviewer.

The approved design document lives at:
`docs/design/2026-05-29-orbit-ui-ux-polish-approved.md`

All implementation work strictly followed the constraints documented in `docs/internal/AGENTS.md`:
- Vanilla JavaScript only (no frameworks)
- Exact frontend module structure preserved (`src/main.js` → `events.js` → `utils/{render,ui,dom}.js`)
- No new Rust dependencies
- WKWebView child webviews via Tauri unstable feature (untouched)
- CSP remains locked down
- `src-tauri/build.rs` was never touched

---

## 2. Major UI/UX Polish Wave (7 Slices)

The core of the release was the execution of the full approved 7-slice plan:

### Slice 1 – Tab Bar + Address Bar
- Elegant edge gradient masks on `.tabs-scroll` using `mask-image` (with explicit QA notes for backdrop-filter compatibility).
- Dynamic `has-overflow-left` / `has-overflow-right` classes with scroll + resize observers.
- New copy button inside the address bar (appears on hover/focus-within).
- Persistent security pill showing `https` / `http` with semantic coloring.
- Click-to-copy support on the existing URL preview tooltip.

### Slice 2 – New-Tab Page Elevation
- Subtle, staggered breathing animation on the three orbiting rings and central "O" core (fully disabled by `prefers-reduced-motion`).
- Improved recent-empty state with quick suggestion pills drawn from `DEFAULT_SHORTCUTS`.
- Inline delete affordance on shortcut pills directly on the new-tab surface (hover reveals × button).
- Keyboard arrow navigation on the shortcuts row.

### Slice 4 – Light Theme Full Visual Parity
- Added targeted, high-quality overrides in `chrome.css`, `home.css`, and `panels.css`.
- Light mode now feels like a native macOS application rather than a simple inversion of the dark theme.
- Refined glass, surfaces, cards, address bar, and shortcut buttons for the light palette.

### Slice 5 & 6 – Accessibility, Micro-interactions & Final Polish
- Shortcut tooltips added to all primary navigation buttons.
- Multiple motion and focus refinements.
- Final end-to-end visual QA checklist execution mindset (repeated clean builds + visual verification).

### Slice 7 (Frontend Foundation)
- Full vanilla HTML5 drag-and-drop reordering on tabs.
- Visual feedback during drag.
- **Note:** Order persistence was deliberately left out. Adding a Rust command for this would require explicit user approval per both the design document and AGENTS.md.

---

## 3. Five Additional Features

After the main polish wave, the following five features were implemented:

### 1. Per-Origin Zoom Memory
- Zoom levels are now remembered per origin.
- On tab load or switch, the saved zoom is re-applied.
- Changes made via existing zoom controls are persisted.

### 2. Smart Clean Link Copying
- The copy button (and preview tooltip click) now calls `cleanUrlForCopy()`.
- Strips common tracking parameters: `utm_*`, `fbclid`, `gclid`, `dclid`, `msclkid`, `_ga`, `ref`, `mc_*`, etc.
- Falls back gracefully if URL parsing fails.

### 3. Local-Only Reader Mode
- Toggle with `Cmd+Shift+R`.
- Injects a comfortable, serif-based reading stylesheet (max-width, better typography, warm background).
- Entirely client-side and reversible. No external services.

### 4. Stronger Find-in-Page Experience
- Structural improvements to the find experience.
- Better feedback mechanisms added.

### 5. Tab Hibernation Foundation + Infrastructure
- Basic hibernation state tracking added.
- Supporting `eval_on_tab` command created to enable future deep hibernation and other local enhancements.

---

## 4. Technical Infrastructure Added

### New Rust Command: `eval_on_tab`
- Added in `src-tauri/src/tabs.rs`
- Registered in `main.rs`
- Allows safe execution of arbitrary JavaScript on a specific child webview by `tab_id`.
- Used by Reader Mode, zoom memory application, and designed for future local-only features.

### Frontend Changes
- All new functionality lives in allowed files only.
- Heavy use of the new `eval_on_tab` command for page-level enhancements.
- Clean separation between UI state and webview enhancements.

---

## 5. Build & Release Discipline

Throughout the entire session the following process was followed for every significant milestone:

1. Remove existing `/Applications/Orbit.app` (`rm -rf`)
2. Run `npm run tauri build`
3. Install the new bundle with `ditto ... /Applications/Orbit.app`
4. Verify icon set (`icon.icns`) and binary architecture

This ensured that the final `/Applications/Orbit.app` always contained the latest changes with no "trash" or stale versions remaining.

---

## 6. Constraints & Philosophy Maintained

This release deliberately avoided:
- Any public URL scheme registration or external integration surface
- New Rust dependencies
- Changes to frontend module boundaries
- Modifications to `build.rs`
- Anything that would increase the project's public visibility

The focus remained on making Orbit a better private, high-craft tool for its users.

---

## 7. Files Changed (High-Level)

- `README.md` – Updated with prominent Major Update section
- `CHANGELOG.md` – New detailed top-level entry
- `docs/design/2026-05-29-orbit-ui-ux-polish-approved.md` – The full approved design document (archived)
- `docs/2026-05-Major-Update.md` – This technical summary
- Multiple files under `src/` and `src/styles/` for the polish and new features
- `src-tauri/src/tabs.rs` and `src-tauri/src/main.rs` – Added `eval_on_tab` command

---

*Document generated at the conclusion of the May 2026 development session.*