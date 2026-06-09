# src/styles — Visual System

## Purpose

Owns Orbit's CSS visual system: design tokens, reset/base styles, browser chrome, new-tab/home surface, panels, toasts, dropdowns, find bar, and responsive visual polish.

## Ownership

- `base.css` — CSS variables, fonts, reset, global app surface.
- `chrome.css` — titlebar, tabs, navbar, address bar, buttons, browser chrome layout.
- `home.css` — new-tab page, logo, shortcuts, search surface, empty-home experience.
- `panels.css` — dropdowns, history/bookmark panels, settings, find bar, toasts.
- `../styles.css` imports the files in order; keep it as the entrypoint.

## Local Contracts

- Dark mode must stay first-class; light mode must not be an afterthought.
- Premium utility feel beats generic template UI.
- Avoid inner rectangular outlines inside rounded controls unless intentionally designed.
- Keep spacing, radius, focus, and hover states coherent across chrome, home, and panels.
- Do not hide overflow problems with broad `overflow: hidden` unless the component actually owns clipping.
- Visual changes must preserve accessibility: visible focus states, readable contrast, and usable hit targets.

## Work Guidance

- Chrome/nav/tab defects usually belong in `chrome.css`.
- New-tab search/shortcuts defects usually belong in `home.css`.
- Dropdown/panel/modal/find/toast defects usually belong in `panels.css`.
- Token-level theme changes belong in `base.css` and must be checked in both themes.
- If CSS changes affect webview vertical position or overlay height, inspect `src/main.js` overlay sync and `src-tauri/src/layout.rs`.

## Verification

- Visual QA: `npm run qa:visual`
- Build sanity: `npm run build`
- Full gate when layout behavior changed: `npm run check`

## Child DOX Index

None.
