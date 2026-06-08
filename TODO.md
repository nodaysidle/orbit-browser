# TODO

Current remaining issues after the Orbit Browser audit-fix hardening pass.

## 1. App is ad-hoc signed, not notarized

**Issue:**
macOS Gatekeeper rejects the packaged app with `spctl` because the app is ad-hoc signed and not Apple-notarized.

**Impact:**
Users may see a macOS security warning when opening the downloaded DMG/app outside a developer machine.

**Solution:**
- Enroll/use an Apple Developer account.
- Configure Developer ID signing certificates.
- Build the release DMG with Developer ID signing.
- Submit the app for notarization with `notarytool`.
- Staple the notarization ticket to the `.app` and `.dmg`.
- Verify with:

```bash
spctl --assess --type execute --verbose /Applications/Orbit.app
xcrun stapler validate /Applications/Orbit.app
```

**Status:**
Backlog / distribution upgrade. Not blocking the current internal release gate.

## 2. `cargo audit` is not available in the current environment

**Issue:**
The Rust dependency security audit was not run because `cargo-audit` is not installed.

**Impact:**
Known RustSec advisories may be missed until the dependency audit tool is installed and wired into release checks.

**Solution:**
Install `cargo-audit` and add it to the verification gate:

```bash
cargo install cargo-audit
cargo audit
```

If advisories appear:
- upgrade affected crates when possible;
- document unavoidable advisories with rationale;
- rerun `npm run check` and the Tauri build after dependency changes.

**Status:**
Backlog / release hygiene.

## 3. Internal audit files are still loose at repository root

**Issue:**
`AUDIT.md` and `AUDIT_PROMPTS.md` exist as local untracked root files.

**Impact:**
They are useful internally, but publishing them at root would make the public repository look more like an internal workbench than a polished product repo.

**Solution:**
Choose one of these paths:

- Keep them local and untracked if they are only internal working notes.
- Move them to `docs/internal/` if they should be versioned.
- Remove them from the migration checkout if they are no longer needed.

If versioned, prefer:

```txt
docs/internal/AUDIT.md
docs/internal/AUDIT_PROMPTS.md
```

**Status:**
Pending product/repo-presentation decision.

## 4. No CI release gate is wired yet

**Issue:**
The full local verification gate passes, but there is no confirmed GitHub Actions gate for every PR/release.

**Impact:**
Future regressions could merge if checks are only run manually.

**Solution:**
Add a GitHub Actions workflow that runs at minimum:

```bash
npm ci
npm run check
```

For release branches/tags, add:

```bash
npm run tauri build -- --bundles dmg
```

Optional later:
- cache Rust/npm dependencies;
- upload DMG artifacts from tagged builds;
- run `cargo audit` after tool installation.

**Status:**
Backlog / automation hardening.

## 5. Manual smoke test is still shallow

**Issue:**
The app launches successfully from the DMG, but the smoke test does not yet exercise a full browser workflow end-to-end.

**Impact:**
Launch success proves packaging is valid, but does not prove tab persistence, keyboard navigation, or session restore work in the installed app.

**Solution:**
Create a repeatable smoke checklist or automated UI smoke script covering:

- launch from installed `.app`;
- open tab;
- close tab;
- reorder tabs;
- quit and relaunch;
- verify session restoration;
- verify keyboard tab navigation;
- verify no console/runtime errors.

**Status:**
Backlog / QA hardening.
