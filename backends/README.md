# Backend ownership

Apple and Android compose one process-owned Rust service. Hosts execute OS
Bluetooth, permissions, location, lifecycle, background execution, scoped file
presentation, theme and UI. Rust owns product policy/storage, protocol processing
and authenticated loopback dispatch. WebViews and listeners borrow that owner.

| Area                                            | Guide                                   |
| ----------------------------------------------- | --------------------------------------- |
| Shared service, protocol and persistence owners | [core/README.md](core/README.md)        |
| Apple adapters, signing and packaging           | [apple/README.md](apple/README.md)      |
| Android adapters and SDK setup                  | [android/README.md](android/README.md)  |
| SQL and session storage                         | [sql/README.md](sql/README.md)          |
| Canonical and physical verification             | [conformance](../conformance/README.md) |

## Protocol and write boundaries

Each connection has one shared request scheduler. Native Bluetooth adapters
execute its ordered writes; they do not create a parallel request path.
`DeviceMutationCoordinator` owns admission for device mutations. The frontend
uses published readiness and completion rather than inferring support or success.
An acknowledgement does not replace required configuration readback; shutdown
acceptance does not establish observed power-off and must not cause retries.

Compatibility catalogs and source-derived fixtures are the release inventory:
[VESC](core/vesc-compatibility/src/lib.rs),
[Refloat](core/refloat-compatibility/src/lib.rs) and
[BMS fixtures](fixtures/vesc-bms/README.md). Do not duplicate version tables,
schema fingerprints, field counts or command layouts in this guide.

Two upstream quirks matter when auditing identities:

- VESC Express VBMS32 reports custom-module type `2`; hardware type alone neither
  identifies nor authorizes a BMS.
- Audited stable Refloat releases can report a suffix repeating the patch number
  because of upstream Makefile extraction. This is a specific audited case,
  not permission to accept arbitrary suffixes, betas or future releases.

Current beta2 support is temporary: replace live beta admission with stable 1.3.1
after its audit, preserving supported saved histories and Tunes. The
[beta2 checkpoint](../conformance/physical-checkpoints/refloat-1-3-1-beta2.md)
owns its verification outcome.

## Auditing another VBMS wire version

1. Identify exact upstream commits under `vendor/` for firmware, Express and VESC Tool.
2. Audit identity, command-96 field order, scales, optional tails and bounds byte for byte.
3. Add immutable fixtures with provenance under [fixtures/vesc-bms](fixtures/vesc-bms/README.md).
4. Update the shared decoder and valid/malformed vectors, including route and error checks.
5. Update contract/mock and canonical scenarios together if public semantics change.
6. Run affected decoder/consumer and conformance checks, then the relevant physical
   interoperability checkpoint. Record unavailable required hardware.

Never broaden compatibility by major version, hardware type, prefix matching or
successful command probing alone.

## Debugging and presentation checks

For ride timing problems, add a marker, finish the ride and export its archive.
Inspect its diagnostic rows without modifying it:

```sh
sqlite3 'file:ride.floatview?immutable=1' \
  'SELECT record_json FROM session_diagnostic ORDER BY sequence;'
```

Sort captured traces by their timestamps rather than insertion order. Timing
summaries and subscriber work are not proof of completed paint or native rendering.
Use [diagnostic instructions](../docs/diagnostics.md) for incident ZIPs; implementation
bounds and event fields live in the collectors and their tests.

Keep native launch canvases aligned with the frontend theme; do not hide connection
errors behind a splash delay. Run `npm run startup` from `conformance/` after
`just build`, and `npm run startup:android` for Android recreation instrumentation.
Cold-launch in light/dark themes and check foreground/WebView reconstruction when
startup presentation changes. The connected-iPhone command is in the
[Apple guide](apple/README.md#startup-recovery).
