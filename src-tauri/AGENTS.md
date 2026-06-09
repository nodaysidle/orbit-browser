# src-tauri — Tauri native shell

## Purpose

Owns Tauri configuration, native Rust backend, capabilities, icons, and bundle settings.

## Ownership

- `Cargo.lock`
- `Cargo.toml`
- `build.rs`
- `capabilities`
- `entitlements.plist`
- `gen`
- `icons`
- `src`
- `tauri.conf.json`

## Local Contracts

- Do not add Rust dependencies without explicit approval.
- Do not change signing, bundle, entitlement, or release behavior unless requested.
- Keep native commands deterministic and error paths user-visible.
- Preserve WKWebView child webviews and the Tauri child-webview/unstable architecture.
- Do not relax CSP or widen permissions in `tauri.conf.json` without explicit approval.
- Do not touch `build.rs` unless the user explicitly names `src-tauri/build.rs`.

## Work Guidance

- Read this file after the root `AGENTS.md` before editing this subtree.
- Prefer extending existing modules/files over creating parallel duplicate systems.
- Update this `AGENTS.md` only when durable ownership, contracts, or verification guidance changes.

## Verification

- Rust/Tauri checks from root package/Cargo manifest when backend changes.
- Use `npm run check` for normal backend changes.
- Use `npm run tauri -- build --bundles dmg`, codesign verification, and runtime smoke when packaging, webview, window, or release-adjacent behavior changes.

## Child DOX Index

- `src-tauri/src/AGENTS.md` — Rust backend implementation.
