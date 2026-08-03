# Focused verification

The final software gate completed successfully on 2026-08-02:

- `frontend`: `npm run check`, `npm test` — 91 tests, no diagnostics.
- `conformance`: `npm run check`, `npm test`, `npm run mock` — 138 tests, no failures.
- Apple: `swift test` and macOS loopback conformance — 69 tests, no failures.
- Android: `./gradlew testDebugUnitTest`, `./gradlew connectedAndroidTest` — 8/8 instrumented tests on the API-35 emulator and 8/8 on the Galaxy S9.
- `just verify`: frontend build, macOS tests, iOS simulator build/conformance, Android emulator conformance, and audit all passed.
- `openspec validate performance-session-visualisations --type change --strict --no-interactive` and `git diff --check` passed.

The physical reports are `ios-physical.json`, `android-physical.json`, and `device-identities.json`. They contain the nine-scenario BLE matrix and exact device identities. The shared physical runner does not expose browser performance marks, SVG DOM counts, long-task telemetry, or a three-run empty-feature-cache UI harness, so those UI-stage measurements are explicitly not inferred from scenario duration.

The iOS session-review checkpoint is complete for the connected iPhone 12. `ios-session-review.json` records three clean-cache initial loads, native and frontend stage timings, cache/SVG counts, gesture-preview latency, settled requests, and the observed diagnostic target misses. The Android checkpoint is now accepted. The browser interaction scenarios are covered by `frontend/src/lib/features/sessions/browser-interactions.spec.ts`; the broader software measurement matrix is intentionally deferred because the mobile benchmarks are sufficient for this change.

The Android session-review checkpoint is accepted in `android-session-review.json` for the connected Galaxy S9. The archive imports and opens, the touch FAB is visible and cycles through interaction modes, and zoom/pan previews are responsive. Three 150k runs miss the one-second initial and 250-millisecond settled diagnostic targets; those misses are accepted for this older device. The remaining browser interaction coverage is now recorded in the frontend scenario suite; no additional software benchmark matrix is required for this change.
