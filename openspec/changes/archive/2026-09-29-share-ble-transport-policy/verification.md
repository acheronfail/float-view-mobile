# Verification and ownership evidence

## Selected boundary

`backend-service/src/ble.rs` is the single transport owner. The existing process
service owns its lifetime, connection generation, operation IDs, queue, profile
selection and timer entries. The existing C/JNI JSON exchanges carry profile,
notification and write-progress facts; no exported binding symbol was added.
The application connection-attempt deadline also rejects late setup results.
Write completion cancels the current timer; cancellation clears both queued
payloads and queued OS write/setup effects. A clock expiry publishes the changed
connection even when the incoming fact ordinarily suppresses snapshot publication.

Native callback correlation includes the original service lifetime and connection
generation. Saturated callback delivery retains an affected connection token in
a fixed cleanup slot rather than attributing overflow to a replacement connection.
Internal write-progress facts do not create public snapshot churn.

Apple uses a separate CoreBluetooth delegate/object lifetime for each attempt;
callbacks cannot be relabelled with a new generation for the same UUID. The
scanner retains no discovered peripheral registry: connection retrieves the
peripheral by its OS UUID. Each connection retains one peripheral, two
characteristics, one setup ID and at most one current OS write. Android keeps at
most 50 discovered device objects, an attempt-owned GATT callback, one setup ID,
one current write ID and one cancellable pacing continuation. Android callbacks
use the supplied main handler directly, avoiding another application callback
queue before the existing 64-slot service admission gate.

Android keeps ATT MTU 23, a 20-byte ceiling and a 20 ms no-response delay. Apple
reports both mode-specific negotiated ceilings and uses OS no-response readiness;
Rust selects with-response when available and caps payloads at 244. Missing
Android CCCD remains supported only after local notification routing succeeds.
Neither native host retains the portable policy helper, logical frame queue or
setup timer. The ownership audit rejects their reintroduction.

The conformance relay now owns simulated profile/notification/progress facts.
Native relay adapters only translate and correlate those facts; no protocol or
capability simulation was added to the hosts or ESP32 firmware.

## Verification

- `just verify` passed: clean Apple packages, four-ABI Android Debug/Release
  packages, all unit suites, direct/macOS/iOS/Android canonical conformance,
  source ownership and production simulation exclusion.
- Shared service: 24 unit tests plus two conformance-owned OS-exchange trace tests
  passed, including exact byte order, both modes, atomic chunk/byte arithmetic,
  stale/duplicate results, setup/write expiry, clock publication and saturation.
- Conformance TypeScript checking and all 195 software tests passed.
- Workspace-wide warnings-denied Rust lint and strict OpenSpec validation passed.
- Built Apple archive exports exactly the eight application-runtime C symbols.
- The final complete `just verify` rerun passed on the final implementation,
  including generation-qualified cancellation and fixture payload measurement.
  An earlier rerun timed out during Android emulator bootstrap; its standalone
  retry and the subsequent complete gate passed without implementation changes.
  Physical acceptance and final cancellation regressions passed on all hosts.

## Audit method

Compare touched `.rs`, `.swift` and `.kt` production paths against `fae14c1f`,
excluding native test directories, conformance-only relay adapters,
`ble/tests.rs`, and trailing Rust `cfg(test)` modules on both sides. Final
production source: **3,608 to 3,287 lines (-321)**. Including the gated relay
adapters on both sides still reduces source from 3,936 to 3,692 (-244).
The coarse repository audit also counts inline Rust test bodies; newly added
fault coverage is deliberately separated from this production comparison.

Workspace remains 28 members with exactly two binding families (eight C and seven
JNI exported declarations). Cargo.lock, frontend contract, SQL and persistent
formats are unchanged. The new conformance Rust test target is test-only and
adds no packaged artifact or dependency.

## Future Linux adapter

A Linux adapter reports UART advertisement presence and exact opaque identity,
required profile availability, per-mode positive payload ceilings, notification
setup outcomes and correlated write progress. It executes scan/connect/cancel,
notification enable and one chunk effect. Capture service lifetime, generation
and operation at issuance, implement OS readiness locally, and preserve bounded
callback admission plus cleanup. Do not implement frame segmentation, a logical
write queue, setup timeout, reconnect policy or public snapshot construction.

## Additional lint observations

The required `just verify` gate passed. An additional `just lint` run reached
four pre-existing frontend ESLint errors: `prefer-const` in
`frontend/backend-mock/index.ts:277` and unkeyed each blocks in
`BmsView.svelte:46,85,98`. Frontend Prettier passed. The separate repository-wide
Markdown/TypeScript formatter reported 40 pre-existing OpenSpec files; none are
this change's implementation or planning files. Swift formatting, Rust formatting,
and `git diff --check` passed. These unrelated files were not rewritten.

An extra sandboxed whole-workspace Rust test invocation hit loopback bind
restrictions; the same transport tests passed inside the approved full gate.
The final focused backend-service run passed all 26 tests without network access.

Selected physical BLE checkpoints and final cancellation regressions passed on
all hosts. See the [historical summary](../../../../conformance/physical-checkpoints/2026-09-29-share-ble-transport-policy.md)
for scope and limits. Detailed run reports are retained locally.


## Requirement evidence

| Requirement                      | Deterministic evidence                                                                                                           | Physical evidence                                                                              |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Shared capability-driven policy  | `filtering_and_profile_are_independent`, OS-exchange missing-notify trace                                                        | Saved/guest discovery and complete GATT setup on all hosts                                     |
| Atomic bounded admission         | `atomic_admission_includes_current_chunk`, checked byte/chunk limits and no emitted prefix                                       | Long direct/CAN configuration writes; fixture-observed host payload limits                     |
| Qualified completion and cleanup | Duplicate/foreign result traces, lifetime/sequence checks, saturation cleanup, write failure tests                               | Same-device reconnect on each host; final qualified-disconnect regressions                     |
| Bounded setup and progress       | Late setup and write-result deadlines, clock-triggered snapshot publication                                                      | Normal setup and sustained telemetry; deliberate missing OS callbacks remain software coverage |
| Native selection                 | Deleted portable helpers/queues/timers, tightened ownership audit, packaged simulation exclusion, unchanged symbols/dependencies | Production macOS, Android API 29 and iPhone BLE plus mobile background continuity              |

No-response physical flow control, missing CCCD and deliberately duplicated or
missing OS callbacks are not claimed as hardware passes. The fixture selected
with-response; both modes and result ordering are covered deterministically.
