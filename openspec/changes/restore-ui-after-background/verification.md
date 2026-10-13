# Verification

## Delivered behavior

One typed version-1 checkpoint survives listener/WebView replacement in shared
process memory. It restores the route, Live mode and allowlisted ordinary
surfaces from current data, while keeping existing background teardown.
Explicit incoming destinations override a different saved destination.
Missing targets fall back; operational failures retain the checkpoint and offer
retry. Cancelling startup keeps the gate closed until new navigation completes,
so late restore responses cannot redirect the user. Saves are deduplicated and
bounded to one in flight plus one latest replacement, without periodic retries.

No schema, archive, native binding, host lifecycle, durable storage or dependency
changes were required. Editors reopen current values; drafts and action
confirmations are intentionally excluded. Detail-load failures now distinguish
missing targets from operational failure so a transient error does not discard
the user's place.

## Software results

Commands below use the repository's existing tools. All final checks passed.

| Area                | Checks and evidence                                                                                                                                                                                                                                                                                                                                               |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Frontend            | `npm test -- src/lib/backend/client.spec.ts src/lib/app/ui-restoration.spec.ts src/lib/backend/ui-checkpoint.spec.ts src/lib/backend/state.spec.ts src/lib/backend/mock-handler.spec.ts`; 52 tests passed before the final additional malformed-checkpoint client test, followed by the client file rerun with all 8 tests passing (53 tests across these files). |
| Types/build         | Frontend `npm run check` (no errors/warnings), `npm run build`; conformance `node node_modules/typescript/bin/tsc --noEmit`. The build retains the existing bundle-size advisory.                                                                                                                                                                                 |
| Format/lint         | Prettier check and ESLint for changed frontend sources; Prettier for changed TypeScript/Markdown/Svelte files; `cargo fmt --all --check`; `git diff --check`.                                                                                                                                                                                                     |
| Service/server      | `cargo test --locked --offline -p float-view-backend-service -p float-view-loopback-server`; atomic validation, UTF-8/encoded-size bounds, clearing, fresh-process emptiness, no revision/events and listener stop/start retention are covered. Loopback tests ran with network permission.                                                                       |
| Native consumers    | `cargo test --locked --offline -p float-view-application-runtime-host -p float-view-application-runtime-apple-ffi -p float-view-application-runtime-android-jni`; existing ownership, ABI and binding consumer tests passed.                                                                                                                                      |
| Rust lint           | `cargo clippy --locked --offline -p float-view-backend-service -p float-view-loopback-server --all-targets -- -D warnings`.                                                                                                                                                                                                                                       |
| Canonical contract  | Conformance `node --strip-types --test test/contract.test.ts` (3 passed) and `node --strip-types src/run.ts direct` (passed). New methods run through the canonical mock/direct suite, including rejected records and client replacement retention; existing authentication/transport scenarios still apply.                                                      |
| Browser restoration | `node --strip-types --test test/ui-restoration.browser.ts`, Chromium and WebKit: actual capture/recreation, detailed Live/charts, session restoration, missing session, each supported surface including BMS, board-history surfaces, failure/retry, cancelled startup/late responses, pending shutdown and flywheel precedence; no mutating request is replayed. |
| Browser regressions | `test/live-menu.browser.ts`, `test/full-screen-overlays.browser.ts` and `test/current-ride.browser.ts`, both engines: pointer/keyboard activation, nested confirmations, lighting/pending controls, explicit navigation, active review/following and native recording independence. Final affected browser runs passed.                                           |
| Planning            | Strict OpenSpec change validation passed after the explicit-destination scenario was added.                                                                                                                                                                                                                                                                       |

The initial browser iterations identified reactive cloning and startup redirect
ordering problems; both were corrected and the affected scenarios rerun.

## Native and physical coverage limitation

This Linux host has no Xcode/Apple tooling, Android SDK/ADB/JVM tools or connected
iOS device available. Swift/Gradle host integration runs and physical iOS
background/foreground validation were not performed. Shared Rust and existing
C/JNI consumer checks establish the software boundary; browser replacement does
not establish OS suspension, memory reclamation, radio/background timing or
physical recording continuity.

Task 4.2 records the explicitly permitted unavailable-hardware outcome. A future
scoped physical iOS check should background from a session detail screen and an
ordinary Live panel, confirm actual WebView/server teardown and restored
destination/mode/surface on return, and observe permitted native recording and
connection continuity. No physical success is claimed by this archive.
