#!/bin/bash
set -euo pipefail

cat <<'CHECKLIST'
=== Orbit Smoke Test ===

Manual installed-app QA checklist:

  1. Open Orbit.app from /Applications or the mounted DMG.
  2. Press ⌘+T — a new tab opens.
  3. Type example.com in the address bar and press Return — page navigates.
  4. Press ⌘+T — a second tab opens.
  5. Drag tab 2 before tab 1 — visual order updates.
  6. Press ⌘+W — the active tab closes without leaving a blank chrome state.
  7. Press ⌘+Q — Orbit quits cleanly.
  8. Reopen Orbit.app — session restores the expected tab and order.
  9. Tab through browser chrome — focus rings are visible.
  10. Use ArrowLeft/ArrowRight in the tab bar — active tab switches.
  11. Press ⌘+Shift+H — home/new-tab page opens.
  12. Check Console.app for orbit: errors — none expected.

Pass criteria: all 12 steps complete without visible error, data loss, or console errors.

Future automation target: replace this checklist with @tauri-apps/driver or Playwright coverage.
CHECKLIST
