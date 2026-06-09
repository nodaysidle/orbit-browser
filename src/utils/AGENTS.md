# src/utils — Frontend Helpers

## Purpose

Owns small reusable frontend helpers used by the vanilla JS runtime: DOM node construction, icons, rendering, modals, toast/error reporting, URL normalization, theme helpers, and navigation snapshots.

## Ownership

- `dom.js` — element/icon factory helpers.
- `render.js` — tabs, history, bookmarks, shortcuts, and list rendering.
- `ui.js` — URL normalization, search engine normalization, theme helpers, navigation title/snapshot helpers.
- `modal.js` — modal lifecycle helpers.
- `toast.js` — toast/error presentation and unhandled rejection handling.

## Local Contracts

- Helpers must stay framework-free and browser-native.
- Rendering helpers should return predictable DOM structures that tests can inspect.
- URL/theme normalization must be deterministic and side-effect-light.
- Keep helpers focused; do not move app state ownership out of `main.js` unless explicitly refactoring.

## Work Guidance

- Add or update Node unit tests when changing output shape or normalization behavior.
- Prefer extending existing helper functions over creating duplicate helpers in `main.js`.
- Keep icon names and SVG output centralized in `dom.js`.

## Verification

- Targeted: `npm test`
- Build sanity: `npm run build`

## Child DOX Index

None.
