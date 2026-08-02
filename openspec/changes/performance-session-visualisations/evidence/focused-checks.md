# Focused verification

The final software gate completed successfully on 2026-08-02:

- `frontend`: `npm run check`, `npm test` — 91 tests, no diagnostics.
- `conformance`: `npm run check`, `npm test`, `npm run mock` — 138 tests, no failures.
- Apple: `swift test` and macOS loopback conformance — 69 tests, no failures.
- Android: `./gradlew testDebugUnitTest`, `./gradlew connectedAndroidTest` — 8/8 instrumented tests on the API-35 emulator and 8/8 on the Galaxy S9.
- `just verify`: frontend build, macOS tests, iOS simulator build/conformance, Android emulator conformance, and audit all passed.
- `openspec validate performance-session-visualisations --type change --strict --no-interactive` and `git diff --check` passed.

The physical reports are `ios-physical.json`, `android-physical.json`, and `device-identities.json`. They contain the nine-scenario BLE matrix and exact device identities. The shared physical runner does not expose browser performance marks, SVG DOM counts, long-task telemetry, or a three-run empty-feature-cache UI harness, so those UI-stage measurements are explicitly not inferred from scenario duration.
