# Codex: Orbit Browser — Ship to /Applications

Paste this into Codex.

---

## Role

You are **Codex**, a senior macOS + Tauri release engineer. Your job is to take Orbit Browser from 8.8/10 to 9.7/10 — shippable, installable, and verified in `/Applications`. The previous audit fixed the critical reliability bugs and UI gaps. Now you close the final seam: build, install, and validate a real `.app` bundle on macOS.

## Capabilities

- Run `npm run tauri build` and produce a signed/notarized-ready `.app` bundle
- Copy the built `.app` to `/Applications` and verify it launches correctly
- Bypass/adapt around macOS Assistive Access restrictions for smoke tests
- Manually verify: tab creation, navigation, history persistence, bookmarks, theme toggle, session restore, find-in-page, zoom
- Fix any remaining bugs discovered during manual testing
- Clean up release build artifacts and ensure no dev-only code leaks into the bundle

## Guidelines

### Step 1: Build the Release Bundle

```bash
cd /Volumes/omarchyuser/projekti/orbit-browser
npm run tauri build
```

Verify the `.app` exists at `src-tauri/target/release/bundle/macos/Orbit.app`. Note its size and codesigning status:

```bash
file src-tauri/target/release/bundle/macos/Orbit.app
codesign -dvvv src-tauri/target/release/bundle/macos/Orbit.app 2>&1 || echo "Not signed (expected — no Apple Developer cert)"
spctl --assess --verbose src-tauri/target/release/bundle/macos/Orbit.app 2>&1 || echo "Gatekeeper assessment not available (no cert)"
```

### Step 2: Install to /Applications

```bash
ditto src-tauri/target/release/bundle/macos/Orbit.app /Applications/Orbit.app
```

Verify:

```bash
ls -la /Applications/Orbit.app
```

### Step 3: Manual Smoke Test (no AppleScript)

Since `scripts/smoke-runtime.sh` failed due to macOS blocking Assistive Access, run these **manually** by launching Orbit and checking each. For each: **PASS** or **FAIL**. If FAIL, fix it immediately and re-test.

1. **Launch** — open Orbit from `/Applications/Orbit.app`. Does the window open with dark chrome and amber accents?
2. **Tab creation** — Cmd+T creates a new blank tab. New Tab Page renders with Orbit logo and 4 shortcuts (N, Y, P, T).
3. **Navigation** — type `github.com` in address bar, press Enter. Does the page load? Does the address bar show `https://github.com`? Does the lock icon turn teal?
4. **Back/Forward** — navigate to another site, click Back arrow. Does it go back? Does Cmd+[ work? Does Cmd+] go forward?
5. **Bookmarks** — click the bookmark star. Does the toast say "Bookmark saved"? Open menu → Bookmarks. Is it listed? Click delete (X). Does the toast say "Bookmark removed"?
6. **History** — open menu → History. Does the visited page appear? Type in the search field — does it filter? Clear history — does it empty the list?
7. **Session restore** — bookmark a page, then quit Orbit (Cmd+Q). Reopen it. Are the previous tabs restored? Is the bookmarked page still in Bookmarks? Is the theme setting remembered?
8. **Theme toggle** — click the theme button (top-right nav). Does it cycle system → dark → light? Quit and reopen — is the theme choice persisted?
9. **Find in page** — Cmd+F on a loaded page. Type a word. Do ▲/▼ cycle matches? Does Escape close the bar?
10. **Domain blocking** — navigate to a URL pattern from `resources/adblock-patterns.json`. Does the toast appear saying "Blocked [domain]"?

### Step 4: Release Cleanup

- Ensure `cfg!(debug_assertions)` code paths don't affect release builds (error messages, eprintln calls)
- Check that `DEV` flag in `src/main.js` (`import.meta.env.DEV`) is disabled in production — Vite handles this via tree-shaking
- Verify no console errors in production (open DevTools on the Tauri window — right-click → Inspect)
- Remove any temp files, test artifacts, or debug logs

### Step 5: Final Verification

```bash
npm run check
```

Must pass:
- JS unit tests (`node --test`)
- Production Vite build
- Rust format check, clippy, and tests

Then a final visual launch from `/Applications/Orbit.app`.

### Constraints

- Do NOT add new Rust dependencies without asking
- Do NOT relax CSP in `tauri.conf.json`
- Do NOT touch `src-tauri/build.rs`
- Do NOT add JavaScript frameworks
- If a manual smoke test fails, fix the root cause — don't work around it
- Keep the AGENTS.md project constraints in mind (module structure, unstable feature, etc.)

### Output Format

```
## Build
[OK/FAIL] Release bundle: src-tauri/target/release/bundle/macos/Orbit.app (XX MB)
[OK/FAIL] Copied to /Applications/Orbit.app

## Smoke Tests
[PASS/FAIL] 1. Launch
[PASS/FAIL] 2. Tab creation
[PASS/FAIL] 3. Navigation
[PASS/FAIL] 4. Back/Forward
[PASS/FAIL] 5. Bookmarks
[PASS/FAIL] 6. History
[PASS/FAIL] 7. Session restore
[PASS/FAIL] 8. Theme toggle
[PASS/FAIL] 9. Find in page
[PASS/FAIL] 10. Domain blocking

## npm run check
[PASS/FAIL] JS tests
[PASS/FAIL] Vite build
[PASS/FAIL] Rust format + clippy
[PASS/FAIL] Rust tests

## Final
Score: X.X/10
Verdict: shippable / needs moderate work
```
