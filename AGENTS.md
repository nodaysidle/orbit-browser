# Orbit-Browser — AGENTS.md

## NODAYSIDLE Law

NODAYSIDLE quality bar: 9.7/10. Ship installable, polished apps. Finished beats fancy. Verified beats assumed.

## Repository Map

If `codemap.md` exists in the project root, read it first for architecture, entry points, directory responsibilities, and data-flow context.

If no root `codemap.md` exists, fall back to:
- this `AGENTS.md`
- the closest child `AGENTS.md` files on the path to the target
- `README.md`
- `PRD.md`, `ARD.md`, `TRD.md`, `TASKS.md`, `TODO.md`, and `CHANGELOG.md` when present
- real entry-point files and config files

## DOX Self-Documentation Contract

This repo uses a DOX-style self-documenting `AGENTS.md` hierarchy for Codex and other coding agents.

Before editing:
1. Read this root `AGENTS.md`.
2. Identify the exact files/folders to touch.
3. Walk the nearest `AGENTS.md` chain from root to target folder.
4. Use the closest `AGENTS.md` for local contracts.
5. Parent rules still apply. Child docs may specialize; they may not weaken parent rules or NODAYSIDLE law.

After meaningful edits:
1. Update the closest relevant `AGENTS.md` only if a durable local contract, file responsibility, verification command, or gotcha changed.
2. Update parent `Child DOX Index` sections only when child docs are added, removed, or repointed.
3. Do not write progress logs, task history, diary notes, or one-off implementation receipts into `AGENTS.md`.
4. Keep docs concise, operational, and true to live files.

Create child `AGENTS.md` files only for durable boundaries with distinct ownership, rules, verification, or architecture. Do not document generated folders such as `dist/`, `node_modules/`, `target/`, `.build/`, `artifacts/`, `.git/`, or release outputs.

## Global Rules

- Make the smallest correct change.
- Do not refactor unrelated code.
- Do not add dependencies without explicit approval.
- Do not change release, signing, notarization, deployment, billing, or credential settings unless asked.
- Do not clean the worktree, delete files, rewrite history, force-push, or remove backups without explicit approval.
- Preserve current stack and architecture unless the task explicitly requires changing them.
- If current code conflicts with these rules, report the conflict before editing.

## Stack Lock

- Tauri 2 desktop browser using Rust backend + vanilla JavaScript frontend.
- Preserve WKWebView child webviews and Tauri child-webview/unstable behavior.
- Keep the frontend vanilla JavaScript only. Do not add React, Vue, Svelte, jQuery, Alpine, or any frontend framework.
- Do not add Electron or replace the native WKWebView/Tauri architecture.
- Do not relax CSP directives or widen `script-src` / permissions without explicit approval.
- Do not add Rust dependencies without explicit approval.
- Do not touch `src-tauri/build.rs` unless the user explicitly names that file.
- multi_agent_v2 is disabled for this project. Work sequentially in one session; do not use `spawn_agent`, `send_input`, `resume_agent`, `wait_agent`, `close_agent`, or any task delegation.

Package scripts are in `package.json`; verify commands from the live file before using them.

## Verification Commands

Use the smallest command that proves the change, then broader gates when feasible.

- npm test
- npm run build
- npm run check
- npm run tauri -- build
- npm run qa:visual
- scripts/premium-visual-qa.sh
- npm run tauri -- build --bundles dmg
- codesign --verify --deep --strict --verbose=2 src-tauri/target/release/bundle/macos/Orbit.app
- scripts/smoke-runtime.sh with `ORBIT_APP_PATH` pointed at the built app when runtime/browser behavior changes.

For docs-only `AGENTS.md` changes, verify with:
- `find . -name AGENTS.md -not -path './.git/*' -not -path './node_modules/*' -not -path './src-tauri/target/*' -not -path './target/*' -not -path './.build/*' | sort`
- `git status --short`
- confirm no product source/config files changed unless explicitly intended.

## Child DOX Index

- `src/AGENTS.md` — Application/frontend source.
- `src-tauri/AGENTS.md` — Tauri native shell.
- `src-tauri/src/AGENTS.md` — Rust backend implementation.
- `scripts/AGENTS.md` — Automation scripts.
- `docs/AGENTS.md` — Project documentation.
- `.github/AGENTS.md` — GitHub automation.
- `tests/AGENTS.md` — Tests.
