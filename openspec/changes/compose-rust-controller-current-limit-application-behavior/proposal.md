## Why

The Rust connection application coordinator now composes discovery, scheduling, telemetry, BMS, mutations, and Refloat configuration, but it still omits the duplicated native application behavior that reads and publishes controller current limits after activation. Composing the existing safe Rust controller-data decoder now completes this remaining connection-scoped read path without adding another temporary Apple FFI or Android JNI boundary.

## What Changes

- Directly compose `vesc-controller-data` into `connection-application-coordinator` as the sole safe owner of command-14 current-limit decoding.
- After exact current controller activation, enqueue one route-correct command-14 request through the existing scheduler at native-equivalent priority and timeout, while standalone BMS activation emits no controller-data work.
- Attribute only the exact current-generation direct/CAN response, retain one bounded redacted current-limit snapshot while ready, publish deterministic state, and clear it on reconnect/reset.
- Fail closed on malformed, stale, mismatched, timed-out, queue-full, cancelled, or repeated completions without failing the connection, publishing private numeric values in diagnostics, or leaving stale limits visible.
- Add source-derived and adversarial direct/CAN/standalone/reset evidence plus dependency, copied-logic, production-isolation, complete software, and all-host physical regression gates.
- Keep Swift and Kotlin selected and unchanged. This change adds no bridge, runtime executor, persistence port, SQLite, session behavior, loopback server, frontend contract change, or OS capability change; later runtime cutovers will delete the native owners and temporary protocol bridge crates.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Require the Rust connection application coordinator to own the complete current-limit request, attribution, decode, publication, and reset lifecycle by directly using the safe controller-data component.
- `platform-hosts`: Record the unselected composed ownership boundary while preserving selected Swift/Kotlin application behavior and every OS capability.
- `backend-conformance`: Require source-derived, adversarial, isolation, software, and Android/iOS/macOS physical evidence before later runtime selection.

## Impact

The change affects only the safe Rust workspace, focused fixtures/tests, root verification recipes, migration documentation, and OpenSpec requirements. `connection-application-coordinator` gains one existing workspace dependency on `vesc-controller-data`; no production package input, exported symbol, native source, frontend API, durable format, or cross-platform behavior changes in this slice.
