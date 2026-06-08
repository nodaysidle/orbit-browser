# Orbit Browser — Audit Fix Prompts (Hermes Iteration)

> Generated from [AUDIT.md](./AUDIT.md) on 2026-06-08.  
> Each prompt is self-contained — paste it into the respective agent to implement the remaining fixes.

---

## 1. OpenAI Codex Prompt

```
You are working on the Orbit Browser project (Tauri 2 + Rust + Vanilla JS).
Perform the following 3 fixes in the codebase:

1. [CSS-1: Focus Ring Contrast]
In `src/styles/base.css`, locate the definition of `--focus-ring` in both light and dark mode rules (around lines 44 and 97).
Increase their contrast/opacity for accessibility:
- Change the dark mode `--focus-ring` value to `rgba(240, 179, 95, 0.6)` (was `0.24`).
- Change the light mode `--focus-ring` value to `rgba(180, 112, 34, 0.55)` (was `0.22`).

2. [P-3: Asynchronous Session Saves]
In `src-tauri/src/tabs.rs`, the function `save_current_session` performs synchronous SQLite writes on the current thread:
```rust
fn save_current_session(app: &AppHandle, state: &BrowserState) {
    // ...
    if let Err(err) = db.save_session(&tabs, active.as_deref()) { ... }
}
```
Refactor this to run asynchronously on Tauri's runtime thread pool so it does not block the UI or command executor:
- Retrieve `app.state::<Db>()` and clone it (or get its pointer) inside a `tauri::async_runtime::spawn` block.
- Move the database save work `db.save_session` into the spawned async task.
Example refactoring outline:
```rust
fn save_current_session(app: &AppHandle, state: &BrowserState) {
    let db = app.state::<Db>().inner().clone(); // Ensure Db implements Clone, or use a reference
    // Alternatively, Db.conn is Mutex<Connection>, which is Arc-like if wrapped or just extract what's needed.
    // Wait, Db cannot be cloned if Connection is not cloneable. But we can retrieve connection or clone app handle.
    let app_clone = app.clone();
    let tabs: Vec<TabInfo> = ...;
    let active = ...;
    tauri::async_runtime::spawn(async move {
        let db = app_clone.state::<Db>();
        if let Err(err) = db.save_session(&tabs, active.as_deref()) {
            report_error(format_args!("failed to save session: {err}"));
        }
    });
}
```

3. [Logic: Tab Order Uniqueness Check]
In `src-tauri/src/tabs.rs`, refactor `validate_tab_order`:
```rust
fn validate_tab_order(
    tabs: &HashMap<String, TabData>,
    ordered_ids: &[String],
) -> Result<(), String> {
    if ordered_ids.len() != tabs.len() || ordered_ids.iter().any(|id| !tabs.contains_key(id)) {
        return Err("invalid tab order".to_string());
    }
    Ok(())
}
```
Add a check to verify that `ordered_ids` contains no duplicate elements:
- Use a `HashSet` to collect visited elements, and return `Err` if a duplicate is found.
Example:
```rust
fn validate_tab_order(
    tabs: &HashMap<String, TabData>,
    ordered_ids: &[String],
) -> Result<(), String> {
    if ordered_ids.len() != tabs.len() || ordered_ids.iter().any(|id| !tabs.contains_key(id)) {
        return Err("invalid tab order".to_string());
    }
    let mut unique_ids = std::collections::HashSet::new();
    for id in ordered_ids {
        if !unique_ids.insert(id) {
            return Err("duplicate tab ID in reorder list".to_string());
        }
    }
    Ok(())
}
```
Add a unit test in the `mod tests` block of `src-tauri/src/tabs.rs` to assert that `validate_tab_order` rejects lists with duplicates.

Run `npm run check` after modifying to verify formatting, compilation, and unit tests.
```

---

## 2. Hermes Agent Prompt

```xml
<context>
  You are an expert Tauri and Rust developer modifying the Orbit Browser codebase.
</context>

<constraints>
  - Follow all AGENTS.md rules.
  - Do not use innerHTML.
  - Maintain concurrency safety.
</constraints>

<task>
  Implement the following three changes:
  1. Increase contrast of `--focus-ring` in `src/styles/base.css` to 0.6 (dark) and 0.55 (light).
  2. Spawn database session saves in `src-tauri/src/tabs.rs` asynchronously using `tauri::async_runtime::spawn` to avoid blocking synchronous SQLite writes.
  3. Validate that the tab list contains no duplicates in `validate_tab_order` in `src-tauri/src/tabs.rs` using a HashSet check, and add a corresponding unit test.
</task>

<output>
  After changes, run `npm run check` to ensure code builds, formats, and passes tests.
</output>
```

---

## 3. Trae Prompt

```
Perform the following updates to the orbit-browser project:

1. Bumps contrast of `--focus-ring` in `src/styles/base.css` to `rgba(240, 179, 95, 0.6)` in dark mode and `rgba(180, 112, 34, 0.55)` in light mode.
2. In `src-tauri/src/tabs.rs`, spawns the `save_session` inside `save_current_session` on an async thread with `tauri::async_runtime::spawn` so it doesn't block the UI thread.
3. In `src-tauri/src/tabs.rs`, updates `validate_tab_order` to reject input lists with duplicates using a HashSet uniqueness check. Adds a unit test named `test_validate_tab_order_rejects_duplicates`.

Run `npm run check` to verify formatting and tests.
```
