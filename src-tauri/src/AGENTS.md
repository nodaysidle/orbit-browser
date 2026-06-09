# src-tauri/src — Rust backend implementation

## Purpose

Owns Tauri command handlers, native state, persistence, filesystem/system integrations, and backend tests.

## Ownership

- `adblock.rs`
- `browser.rs`
- `db.rs`
- `download.rs`
- `layout.rs`
- `main.rs`
- `tabs.rs`

## Local Contracts

- Do not add Rust dependencies without explicit approval.
- Do not change signing, bundle, entitlement, or release behavior unless requested.
- Keep native commands deterministic and error paths user-visible.
- Preserve WKWebView child webviews and the Tauri child-webview/unstable architecture.
- Do not relax CSP assumptions from backend commands or Tauri config.
- Do not touch `../build.rs` unless the user explicitly names `src-tauri/build.rs`.

## Work Guidance

- Read this file after the root `AGENTS.md` before editing this subtree.
- Prefer extending existing modules/files over creating parallel duplicate systems.
- Update this `AGENTS.md` only when durable ownership, contracts, or verification guidance changes.

## Verification

- Rust/Tauri checks from root package/Cargo manifest when backend changes.
- Use `npm run check` for backend changes; add runtime smoke for tab/webview/session/navigation behavior.

## Child DOX Index

None.
