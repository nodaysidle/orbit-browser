# docs — Project documentation

## Purpose

Owns public/internal project docs, plans, audit notes, prompts, and release-supporting documentation.

## Ownership

- `2026-05-Major-Update.md`
- `README.md`
- `design`
- `internal`
- `plans`

## Local Contracts

- Keep docs factual and aligned with live repo state.
- Do not store secrets, raw tokens, or private credentials.
- Mark unknowns as `UNKNOWN` rather than inventing status.

## Work Guidance

- Read this file after the root `AGENTS.md` before editing this subtree.
- Prefer extending existing modules/files over creating parallel duplicate systems.
- Update this `AGENTS.md` only when durable ownership, contracts, or verification guidance changes.

## Verification

- Read changed files back.
- Inspect `git diff --name-only` / `git status --short`.
- For claims about builds, releases, smoke tests, or quality scores, cite live command output or mark the status `UNKNOWN`.

## Child DOX Index

- `docs/internal/AGENTS.md` — Internal agent prompts, audits, handoffs, and operational docs.
