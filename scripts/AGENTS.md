# scripts — Automation scripts

## Purpose

Owns local build, package, smoke, install, QA, and release helper scripts.

## Ownership

- `build-mac.sh`
- `install-app.sh`
- `premium-visual-qa.mjs`
- `premium-visual-qa.sh`
- `smoke-runtime.sh`
- `smoke-test.sh`

## Local Contracts

- Scripts must be safe, deterministic, and scoped.
- Do not delete user data, releases, caches, or installed apps without explicit approval.

## Work Guidance

- Read this file after the root `AGENTS.md` before editing this subtree.
- Prefer extending existing modules/files over creating parallel duplicate systems.
- Update this `AGENTS.md` only when durable ownership, contracts, or verification guidance changes.

## Verification

- Read changed files back.
- Inspect `git diff --name-only` / `git status --short`.
- For JS scripts, run the specific script or a dry-run/syntax-safe equivalent where available.
- For visual QA changes, run `npm run qa:visual` and/or `scripts/premium-visual-qa.sh`.
- For build/install/smoke scripts, do not claim success without live command output from the relevant script.

## Child DOX Index

None.
