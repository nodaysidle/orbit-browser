# 🤖 Hermes Handoff — Orbit Browser Iteration 3

> **Date**: 2026-06-08 | **Author**: Antigravity (Audit Agent)  
> **Context**: Hermes completed two passes on Orbit. This document is a prioritized task list for the next session based on an audit of `TODO.md` and the current codebase state.

---

## ✅ What Hermes Has Completed (Across Both Passes)

Hermes did excellent work. Here's the full scorecard:

| ID | Item | Status |
|----|------|--------|
| D-1 | Schema migration system (`schema_version` table, `CURRENT_SCHEMA_VERSION`, upgrade path) | ✅ Done |
| C-1 | Lock ordering (unified `tabs` → `tab_order` in `close_tab`, `save_current_session`) | ✅ Done |
| C-5 | Double-lock in `sync_visible_webviews` consolidated to single scope | ✅ Done |
| C-6 | Poisoned mutex recovery in `lock_state` and `lock_conn` (both browser + db) | ✅ Done |
| L-3 | `tab_order` rollback on `create_tab` webview failure | ✅ Done |
| F-4 | Tab drag-and-drop persists via `reorder_tabs` command | ✅ Done |
| E-1 | `unhandledrejection` handler with toast notification | ✅ Done |
| E-2 | Session restore errors surfaced via `report_error` (match-based) | ✅ Done |
| AX-1 | ARIA `role="tablist"` / `role="tab"` / `role="tabpanel"` + `aria-selected` + `aria-controls` + `id` linking | ✅ Done |
| CSS-1 | Focus ring contrast bumped to 0.6/0.55 | ✅ Done |
| P-3 | Session saves moved to `tauri::async_runtime::spawn` | ✅ Done |
| Logic | `validate_tab_order` uniqueness check with `HashSet` | ✅ Done |
| CSP | Internal URLs upgraded from `http://` to `https://` (`asset.localhost`, `ipc.localhost`) | ✅ Done |
| Entitlements | Added `entitlements.plist` for JIT/unsigned memory (WKWebView requirement) | ✅ Done |
| Window resize | 16ms throttle on resize handler to prevent frame-rate lock contention | ✅ Done |
| Keyboard nav | Arrow key tab switching in tablist + `Ctrl+Tab` / `Ctrl+Shift+Tab` shortcuts | ✅ Done |
| Shortcut delete | `:focus-visible` style for `.shortcut-delete` button | ✅ Done |
| ARIA linking | Tab buttons get `id="tab-btn-{id}"` and tabpanel gets `aria-labelledby` | ✅ Done |
| Repo cleanup | Moved `AGENTS.md`, `CODEX_BUILD_PROMPT.md`, `PROJECT_BRIEF.md` to `docs/internal/` | ✅ Done |
| `save_session` command | Removed duplicate frontend-facing `save_session` Tauri command (now internal only) | ✅ Done |

**Overall Quality**: Excellent. All 66 Rust tests pass, all 27 JS tests pass, `cargo clippy` is clean, `cargo fmt --check` is clean.

---

## 🔴 What Hermes Still Needs to Do

The following items remain from `TODO.md` and from my audit of the current codebase state. They are ordered by priority.

---

### Task 1 — CI/CD GitHub Actions Workflow (TODO.md #3)

**Why**: No automated gate exists. All checks are manual. A regression can merge silently.

**What to do**:

Create `.github/workflows/ci.yml` with:

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  check:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '22'
          cache: 'npm'
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: npm ci
      - run: npm test
      - run: npm run build
      - run: cargo fmt --manifest-path src-tauri/Cargo.toml --check
      - run: cargo test --manifest-path src-tauri/Cargo.toml
      - run: cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

Optionally add a release job for tagged builds:

```yaml
  release:
    if: startsWith(github.ref, 'refs/tags/')
    needs: check
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '22'
          cache: 'npm'
      - uses: dtolnay/rust-toolchain@stable
      - run: npm ci
      - run: npm run tauri -- build --bundles dmg
      - uses: actions/upload-artifact@v4
        with:
          name: orbit-dmg
          path: src-tauri/target/release/bundle/macos/*.dmg
```

**Effort**: Low  
**Status from TODO.md**: Backlog

---

### Task 2 — Modal Focus: Return Focus to Trigger on Close (AX-2)

**Why**: When opening About or Settings modals, focus is correctly trapped and moved to the first focusable element (Hermes already implemented `bindModalFocusTrap` + `openModal` with fallback focus). However, when the modal is *closed*, focus is returned to `activeModalTrigger` — but only if it's an `HTMLElement`. If the trigger was the keyboard shortcut handler (not a button), `activeModalTrigger` will be `null` and focus goes nowhere.

**What to do**:

In `closeModal()` in [main.js L1035-1047](file:///Volumes/omarchyuser/projekti/_github-migrations/orbit-browser/src/main.js#L1035-L1047):

```javascript
function closeModal(modal) {
  if (!modal) return
  modal.classList.add('hidden')
  if (activeModalFocusHandler) {
    modal.removeEventListener('keydown', activeModalFocusHandler)
  }
  activeModalFocusHandler = null
  activeModal = null
  if (activeModalTrigger instanceof HTMLElement) {
    activeModalTrigger.focus()
  } else {
    // Fallback: return focus to address bar when no trigger element exists
    $('addressInput')?.focus()
  }
  activeModalTrigger = null
}
```

**Effort**: Trivial  
**Status from original audit**: Open (AX-2)

---

### Task 3 — History Size Limit (D-4)

**Why**: The `history` table grows unbounded. After months of use, it could contain hundreds of thousands of rows, slowing down searches and increasing database size.

**What to do**:

In [db.rs](file:///Volumes/omarchyuser/projekti/_github-migrations/orbit-browser/src-tauri/src/db.rs), add a history cleanup function:

```rust
const MAX_HISTORY_ENTRIES: i64 = 10_000;

pub fn trim_history(&self) -> Result<()> {
    let conn = self.lock_conn()?;
    conn.execute(
        "DELETE FROM history WHERE id NOT IN (
            SELECT id FROM history ORDER BY last_visited DESC LIMIT ?1
        )",
        params![MAX_HISTORY_ENTRIES],
    )?;
    Ok(())
}
```

Call `db.trim_history()` on app startup (after `init_schema()`), and optionally every 100 `add_history` calls. Add a unit test asserting that the oldest entries are dropped.

**Effort**: Low  
**Status from original audit**: Open (D-4)

---

### Task 4 — Smoke Test Checklist Script (TODO.md #4)

**Why**: The DMG launches but no repeatable QA script exists to verify tab lifecycle, session restore, or keyboard navigation in the installed app.

**What to do**:

Create `scripts/smoke-test.sh`:

```bash
#!/bin/bash
set -e
echo "=== Orbit Smoke Test ==="
echo ""
echo "Manual steps (automated later with Tauri driver):"
echo ""
echo "  1. Open Orbit.app"
echo "  2. ⌘+T — new tab opens"
echo "  3. Type 'example.com' in address bar — navigates"
echo "  4. ⌘+T — second tab opens"
echo "  5. Drag tab 2 before tab 1 — order updates visually"
echo "  6. ⌘+W — close active tab"
echo "  7. ⌘+Q — quit Orbit"
echo "  8. Reopen Orbit.app — session restored (correct tab, correct order)"
echo "  9. Tab through chrome with keyboard — focus rings visible"
echo "  10. Arrow keys in tab bar — switches between tabs"
echo "  11. ⌘+Shift+H — opens home page"
echo "  12. Check Console.app for 'orbit:' errors — none expected"
echo ""
echo "Pass criteria: All 12 steps complete without error."
```

Later, wire up actual automated E2E tests using `@tauri-apps/driver` or Playwright.

**Effort**: Trivial (checklist), Medium (automated)  
**Status from TODO.md**: Backlog

---

### Task 5 — `cargo audit` Integration (TODO.md #1)

**Why**: No dependency vulnerability scanning is performed. Known RustSec advisories could be missed.

**What to do**:

1. Install `cargo-audit`: `cargo install cargo-audit`
2. Run `cargo audit` in the `src-tauri` directory
3. If any advisories appear, upgrade the affected crates or document the rationale for suppressing them
4. Add `cargo audit` to the CI workflow (Task 1) as an optional step:

```yaml
      - run: cargo install cargo-audit
      - run: cargo audit --manifest-path src-tauri/Cargo.toml
```

**Effort**: Trivial  
**Status from TODO.md**: Backlog

---

### Task 6 — Move/Remove Loose Audit Files (TODO.md #2)

**Why**: `AUDIT.md` and `AUDIT_PROMPTS.md` at the repo root make the public repository look like an internal workbench.

**What to do**:

- Move both files to `docs/internal/` (where `AGENTS.md`, `PROJECT_BRIEF.md`, and `CODEX_BUILD_PROMPT.md` already live):

```bash
mv AUDIT.md docs/internal/
mv AUDIT_PROMPTS.md docs/internal/
```

- Add them to `.gitignore` if they should remain untracked, or commit them to `docs/internal/` if they should be versioned.

**Effort**: Trivial  
**Status from TODO.md**: Pending decision

---

### Task 7 — Debounce/Coalesce Frontend `queueBrowserViewSync` (F-5)

**Why**: `queueBrowserViewSync` fires on every `requestAnimationFrame` during window resize. Even with the new 16ms backend throttle, the frontend still dispatches IPC calls at 60Hz.

**What to do**:

In [main.js L327-333](file:///Volumes/omarchyuser/projekti/_github-migrations/orbit-browser/src/main.js#L327-L333), add a minimum 100ms interval between IPC dispatches:

```javascript
let lastSyncTime = 0
function queueBrowserViewSync() {
  if (state.resizeFrame) return
  state.resizeFrame = requestAnimationFrame(() => {
    state.resizeFrame = 0
    const now = Date.now()
    if (now - lastSyncTime < 100) return
    lastSyncTime = now
    syncBrowserView()
  })
}
```

**Effort**: Trivial  
**Status from original audit**: Open (F-5)

---

### Task 8 — Extract Magic Numbers (F-2)

**Why**: `3200` (toast duration), `190` (toast animation), `12000` (confirm toast timeout), `680` (MAX_OVERLAY_HEIGHT) are hardcoded in `main.js`.

**What to do**:

Add named constants at the top of `main.js`:

```javascript
const TOAST_DURATION_MS = 3200
const TOAST_FADE_MS = 190
const CONFIRM_TOAST_TIMEOUT_MS = 12000
```

Replace the literal values with these constants. `MAX_OVERLAY_HEIGHT` is already a named constant (line 50).

**Effort**: Trivial  
**Status from original audit**: Open (F-2)

---

## Summary Priority Table

| # | Task | Effort | Impact | Category |
|---|------|--------|--------|----------|
| 1 | CI/CD GitHub Actions | Low | 🔴 High (prevents regressions) | Automation |
| 2 | Modal focus fallback on close | Trivial | Medium (a11y) | Accessibility |
| 3 | History size limit | Low | Medium (perf) | Database |
| 4 | Smoke test checklist | Trivial | Medium (QA) | Testing |
| 5 | `cargo audit` | Trivial | Low (security hygiene) | Dependencies |
| 6 | Move audit files to `docs/internal/` | Trivial | Low (repo presentation) | Cleanup |
| 7 | Debounce frontend resize sync | Trivial | Low (perf) | Performance |
| 8 | Extract magic numbers | Trivial | Low (code quality) | Frontend |

---

> **Overall Assessment**: The codebase is now at **9.0/10**. All critical and high-severity items from the original audit are resolved. The remaining tasks are all low-effort polish items. Hermes did exceptional work across two iterations — the concurrency model, schema migration system, accessibility markup, and async session saves are all production-quality implementations.
