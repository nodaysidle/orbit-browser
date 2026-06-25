# src — Application/frontend source

## Purpose

Owns the main application UI/runtime source for this project.

## Ownership

- `events.js`
- `find.js`
- `fonts`
- `main.js`
- `shortcuts.js`
- `styles`
- `styles.css`
- `utils`
- `zoom.js`

## Local Contracts

- Preserve the current vanilla JavaScript frontend stack and component architecture.
- Keep UI polished, accessible, and dark-mode friendly where applicable.
- Do not introduce React, Vue, Svelte, jQuery, Alpine, or any frontend framework without explicit approval.
- Do not weaken CSP assumptions or add inline script patterns that require relaxing CSP.

## Work Guidance

- Read this file after the root `AGENTS.md` before editing this subtree.
- Prefer extending existing modules/files over creating parallel duplicate systems.
- Update this `AGENTS.md` only when durable ownership, contracts, or verification guidance changes.

## Verification

- Frontend/build check from root package manifest when behavior changes.
- Use `npm test`, `npm run build`, and `npm run qa:visual` for UI/runtime-facing changes where feasible.

## Child DOX Index

- `src/styles/AGENTS.md` — CSS/theme/chrome styling contracts.
- `src/utils/AGENTS.md` — Frontend utility/rendering contracts.
