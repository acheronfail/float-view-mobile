# Verification — 2026-10-17

## Implemented behavior

One installation-local preferred board, a 15-second foreground discovery
opportunity sharing the normal scanner, exact saved association matching, manual
suppression, foreground rearming, and preserved non-live navigation are implemented.
The preference is transactionally cleared when its board is removed, including
when later file cleanup fails. Incompatible development storage uses marker 26.

Android configuration replacement retains its foreground visit. macOS window
closure ends the visit and reopening reports actual activation. No new dependency
or native product policy was introduced.

## Software checks

- Affected product-store, backend-service, and loopback-server tests passed with
  `cargo test --locked --offline -p float-view-product-store
-p float-view-backend-service -p float-view-loopback-server` from `backends/core`.
  The final backend-service unit run passed all 76 tests. Store coverage includes
  atomic selection and deletion rollback, cleanup failure after removal, reopen,
  archive isolation, schema mismatch reset, and ordinary corruption preservation.
- Warnings-denied Clippy passed for those crates and all their targets. Rust
  formatting and diff whitespace checks passed.
- Frontend Svelte/type checks, focused unit suites, ESLint on changed frontend
  code, formatting checks, and production build passed.
- Chromium and WebKit coverage exercises primary/non-primary/keyboard switch
  activation, one submission per activation, failed-save presentation, automatic
  connection before checkpoint loading, and preserved Settings navigation.
  Existing discovery and UI restoration browser suites also passed.
- Conformance TypeScript checking and the 17 fixture/target tests passed. A
  pre-existing inference error in the live-menu browser test needed an explicit
  numeric annotation on its request-count checkpoint.
- Full mock and direct conformance passed. Focused native conformance passed on
  macOS, iOS Simulator, and Android emulator using
  `FLOAT_VIEW_CONFORMANCE_SCENARIOS=auto-connect npm run <target>`.
  Android exercises actual background/foreground controls. Headless Apple targets
  establish contract/binding behavior, not physical window or BLE lifecycle.
- Apple host tests passed (23 tests); Android host/JNI unit tests passed with the
  installed Java 21 runtime. Native application builds also succeeded.

## Verification adjustments and limitations

The full native suites reached unrelated location-permission presentation and
could not complete unattended. Focused auto-connect mode avoids those prompts.
Native discovery gets a one-second initial scan allowance rather than racing a
100 ms fixture callback. Android lifecycle detection now recognizes modern
STOPPED records without the removed `stopped` field and terminal DESTROYED records.

Initial browser/loopback checks were blocked by sandbox listening restrictions;
the reported passing runs used local access. An initial overlapping Android
emulator/physical attempt shared the forwarded port and was discarded; final
Android runs were sequential.

## Physical checkpoint remains incomplete


The scoped Android auto-connect fixture run failed to discover the expected
advertisement within its bounded attempts. No fixture connection or automatic
reconnection was established; cleanup completed without actuation.

iOS hardware was unavailable. macOS activation, first-use permission presentation
and remaining mobile lifecycle cases are unverified. Task 5.3 remains open.
Detailed device results and execution/setup history are retained locally under
`.local/physical-testing/`; this is not physical acceptance.
