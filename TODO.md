# TODO

Current remaining issues after Orbit Browser handoff iteration 3.

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

## 3. Automated installed-app smoke coverage

**Issue:**
`scripts/smoke-test.sh` provides a repeatable manual QA checklist, but it is not automated yet.

**Impact:**
Manual QA can catch tab/session/keyboard regressions, but future releases still depend on human execution.

**Solution:**
Replace or extend the checklist with automated coverage using `@tauri-apps/driver` or Playwright once the browser chrome can be driven reliably.

**Status:**
Backlog / QA automation hardening.
