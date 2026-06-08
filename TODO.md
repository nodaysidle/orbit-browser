# TODO

Current remaining issues after Orbit Browser premium release pass.

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
Backlog / distribution upgrade. Not blocking the current internal stress-test gate.

## 2. Rust dependency audit gate

**Issue:**
`cargo audit` now exists in CI, but dependency vulnerability scanning still depends on RustSec advisory availability and the installed `cargo-audit` binary.

**Impact:**
Known RustSec advisories may be missed if the security job is skipped or unavailable.

**Solution:**
Keep the CI security job active and run locally before releases:

```bash
cargo audit
```

If advisories appear:
- upgrade affected crates when possible;
- document unavoidable advisories with rationale;
- rerun `npm run check` and the Tauri build after dependency changes.

**Status:**
Release hygiene.

## 3. Visual QA and tab-reorder smoke coverage

**Status:**
Resolved for local/internal release QA.

**Evidence:**
- `scripts/premium-visual-qa.sh` captures dark and light screenshots through Playwright and reports frame-timing/overflow/focus metrics.
- `scripts/smoke-runtime.sh` now drives keyboard tab reorder in the built app and verifies the persisted session order through SQLite.

**Remaining:**
Manual human stress testing by NDI before any public release.
