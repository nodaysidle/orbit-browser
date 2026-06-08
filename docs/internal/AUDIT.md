# 🔍 Orbit Browser — Follow-up Codebase Audit (Hermes Iteration)

> **Date**: 2026-06-08 | **Scope**: Read-only, review of Hermes fixes in `_github-migrations/orbit-browser`  
> **Stack**: Tauri 2.x · Rust + Vanilla JS · WKWebView · SQLite  
> **Ref Branch**: `task/orbit-audit-fixes` | **Last Commit**: `c7788db` ("Fix Orbit audit hard gates")  
> **Codebase Size**: ~5,280 LOC

---

## Executive Summary

This follow-up audit evaluates the changes implemented by the **Hermes agent (telegram-gpt5.5)** on top of the original codebase. The fixes targeted several high-risk areas identified in the initial audit, including database persistence, concurrency safety, tab order serialization, and frontend accessibility.

The quality of the implemented fixes is **very high**. Hermes successfully mitigated the critical database schema migration issue, resolved potential concurrency deadlocks through strict lock-ordering and poisoned-mutex recovery, resolved ghost entry memory leaks, and added proper ARIA tab roles for accessibility. 

As a result of these improvements, the project is now in a much more stable and production-ready state.

### Score Recalculation

| Area | Previous Score | New Score | Status |
|------|----------------|-----------|--------|
| Architecture & Organization | 8.5/10 | **8.5/10** | ✅ Strong, monolith remains |
| Security | 8/10 | **8/10** | ✅ Good |
| Rust Concurrency & Safety | 6.5/10 | **9/10** | 🚀 Excellent (Deadlocks & poisoning resolved) |
| Error Handling | 7.5/10 | **8.5/10** | ✅ Unhandled rejections caught |
| Database & Persistence | 8/10 | **9/10** | 🚀 Excellent (Migrations added) |
| Frontend Code Quality | 8/10 | **8.5/10** | ✅ Tab drag-and-drop persisted |
| CSS & Theming | 8.5/10 | **8.5/10** | ✅ Polished |
| Accessibility | 7/10 | **8.5/10** | 🚀 Strong (Tab roles implemented) |
| Performance | 7/10 | **7.5/10** | ⚠️ Synchronous DB writes remain |
| Test Coverage | 5/10 | **6.5/10** | ⚠️ Improved coverage for new functions |

**Overall Score: 8.7/10** (Up from **7.5/10**). The browser is now safe to run and update without database breakage or concurrency bricking.

---

## 🛠️ What Was Fixed by Hermes (Commit `c7788db`)

Hermes addressed 6 major areas from the initial audit report:

### 1. Schema Migrations (D-1) — *Critical*
- **Fix**: Added a version tracking system to the database schema in `src-tauri/src/db.rs`.
- **Implementation**:
  - Introduced the `schema_version` table.
  - Set `CURRENT_SCHEMA_VERSION = 1`.
  - Added an upgrade pathway `apply_schema_v1(&conn)`.
  - Added two new unit tests (`test_schema_version_is_initialized` and `test_unversioned_existing_database_migrates`) to verify backward compatibility.

### 2. Lock Poisoning Recovery (C-6) — *High*
- **Fix**: Prevented permanent app bricking if a state lock poisons during a thread panic in `src-tauri/src/browser.rs`.
- **Implementation**:
  - Refactored `lock_state` to intercept `Err(poisoned)` and recover the inner guard using `poisoned.into_inner()`.
  - Added a test case `test_lock_state_recovers_poisoned_mutex` to assert successful recovery.

### 3. Concurrency Deadlocks (C-1 / C-5) — *High*
- **Fix**: Replaced nested locks with clean, sequential lock acquisitions or unified lock orders across `src-tauri/src/tabs.rs` and `src-tauri/src/layout.rs`.
- **Implementation**:
  - Unified lock acquisition in `close_tab` and `save_current_session` to always lock `tabs` before locking `tab_order`.
  - Unified the double-lock pattern in `sync_visible_webviews` (`src-tauri/src/layout.rs`) into a single critical section scope.

### 4. Tab Order Serialization & Persistence (F-4) — *Medium*
- **Fix**: Drag-and-drop tab reordering is now saved and persists across app restarts.
- **Implementation**:
  - Added the `reorder_tabs` command on the Rust backend with safety validation.
  - Hooked up `events.js` and `main.js` to dispatch order updates on `dragend`.
  - Added JavaScript unit tests in `events.test.js` to assert visual reordering propagation.

### 5. Accessibility Tab Roles (AX-1) — *Medium*
- **Fix**: Implemented complete semantic markup for tabs.
- **Implementation**:
  - Added `role="tablist"` to `#tabsContainer` and `role="tabpanel"` to `#newTabPage`.
  - Added `role="tab"`, `aria-selected`, and appropriate `tabindex` to the tab main buttons in `src/utils/render.js`.
  - Configured unit tests in `render.test.js` to verify these attributes.

### 6. Unhandled Promise Rejections (E-1) — *Medium*
- **Fix**: Uncaught async errors on the frontend are now logged and reported to the user.
- **Implementation**:
  - Registered an `unhandledrejection` window listener in `src/main.js` that triggers a toast notification.

---

## ⚠️ Remaining Open Issues & Code Gaps

While the most critical items are resolved, several minor architectural and performance issues remain.

### 1. Performance: Eager and Synchronous Session Saves (P-3)
- **Symptom**: `save_current_session` in `tabs.rs` is called on every tab switch, close, create, and navigation event. It executes a synchronous write to SQLite on the current thread, blocking the tokio/tauri thread pool.
- **Location**: [tabs.rs L15-37](file:///Volumes/omarchyuser/projekti/_github-migrations/orbit-browser/src-tauri/src/tabs.rs#L15-L37)
- **Impact**: Medium. With rapid tab switching, this could lead to minor frame drops or latency in UI updates.
- **Proposed Fix**: Debounce session saves on the frontend or move them to an asynchronous task in Rust using `tokio::spawn`.

### 2. Logic Gap: Tab Order Validation Lacks Uniqueness Verification
- **Symptom**: `validate_tab_order` in `tabs.rs` checks that the length of the ordered IDs matches the map length and that each ID exists, but it does not check for duplicate elements in the input list.
- **Location**: [tabs.rs L760-766](file:///Volumes/omarchyuser/projekti/_github-migrations/orbit-browser/src-tauri/src/tabs.rs#L760-L766)
- **Impact**: Low. A buggy frontend request containing duplicate IDs could corrupt the `tab_order` list with ghost duplicates.
- **Proposed Fix**: Verify uniqueness by checking if all elements in the input list are unique (e.g. using a `HashSet` check).

### 3. Architecture: Monolithic JS and dual-purpose main.rs (A-1 / A-3)
- **Symptom**: `main.js` remains a 1,340-line monolith, and `main.rs` houses database wrappers.
- **Location**: `src/main.js` & `src-tauri/src/main.rs`
- **Impact**: Low. Code is highly structured and passes all unit tests, but refactoring into smaller modules would improve long-term maintainability.

### 4. Accessibility: Faint Focus Ring Contrast (CSS-1)
- **Symptom**: Nav buttons and tabs use custom focus styling (`box-shadow: 0 0 0 3px var(--focus-ring)`). However, `--focus-ring` is configured with a very low opacity (0.24 in dark mode, 0.22 in light mode), making it extremely faint and hard to see.
- **Location**: [chrome.css L341-347](file:///Volumes/omarchyuser/projekti/_github-migrations/orbit-browser/src/styles/chrome.css#L341-L347)
- **Impact**: Medium (Accessibility). Keyboard users will find it difficult to track focus.
- **Proposed Fix**: Increase the focus ring opacity (e.g. to `0.6` or `0.8`) or fall back to a high-contrast native outline.

### 5. Security: Arbitrary script execution (S-3)
- **Symptom**: `eval_on_tab` command evaluates arbitrary strings passed from the frontend.
- **Location**: [tabs.rs L842-848](file:///Volumes/omarchyuser/projekti/_github-migrations/orbit-browser/src-tauri/src/tabs.rs#L842-L848)
- **Impact**: Low (Gated by IPC).
- **Proposed Fix**: Restrict evaluations to pre-compiled scripts or enforce code signing assertions.

---

## Priority Matrix (Remaining Tasks)

```mermaid
quadrantChart
    title Prioritized Next Steps (Remaining Tasks)
    x-axis Low Effort --> High Effort
    y-axis Low Impact --> High Impact
    quadrant-1 "Next Steps"
    quadrant-2 "Refactorings"
    quadrant-3 "Quick Wins"
    quadrant-4 "Long Term"
    "Debounce Session Saves (P-3)": [0.4, 0.75]
    "Validate Uniqueness in Tab Reorder": [0.2, 0.6]
    "Increase Focus Ring Contrast (CSS-1)": [0.15, 0.7]
    "Refactor main.js monolith (A-1)": [0.8, 0.5]
    "Move DB commands from main.rs (A-3)": [0.6, 0.4]
    "Debounce Window Resize Sync": [0.3, 0.4]
```

## Top 4 Recommended Fixes for Next Sprint

1. **CSS-1**: Increase focus ring visibility by bumping the opacity of `--focus-ring` (or adding a dedicated `:focus-visible` offset outline).
2. **P-3**: Move database session saves to a background tokio thread to prevent blocking the async executor during IO.
3. **Logic**: Refactor `validate_tab_order` in `tabs.rs` to assert uniqueness of tab IDs.
4. **A-1**: Extract utility functions or specific subsystems (e.g. `shortcuts.js` or `theme.js`) out of `main.js`.
