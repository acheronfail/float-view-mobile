## Context

Apple and Android now delegate four focused VESC protocol families to safe Rust components, but each backend still implements the same Refloat live-control codec. The duplicated code constructs custom-app-data payloads for package lock, flywheel, two generations of lighting control, and fatal-latch clear, and validates the corresponding lighting response. It is stateless after the caller supplies the compatibility-derived control profile and current lighting state.

Pinned Refloat releases define the command layouts. The repository already contains immutable source-derived product-control requests/responses, native parity tests, a closed conformance decoder, direct and CAN simulation, and physical `control-direct` coverage. This change proves one safe Rust codec against that evidence without selecting it from a host.

## Goals / Non-Goals

**Goals:**

- Add one safe Rust component with an empty production dependency graph dedicated to bounded Refloat live-control payload encoding and lighting-response decoding.
- Match native custom-app-data, package, command, parameter, profile, mask, flag, query, response-bound, and failure behavior.
- Reuse immutable source-derived controls and cover every state combination and malformed class without adding a fixture parser dependency.
- Keep workspace layout, dependency, unsafe-code, privacy, bridge-symbol, packaging, and production-selection audits exact.
- Prove no repository or real-BLE regression through `just verify` and focused Refloat physical scenarios on Android, iOS, and macOS.
- Leave a clean candidate for later, separate Apple and Android bridge/cutover changes.

**Non-Goals:**

- Adding or changing the Apple C ABI, Android JNI ABI, native packaging, or production selection.
- Removing or changing the Swift/Kotlin Refloat control codec, compatibility table, routing, framing, authorization, pending-mutation gate, scheduling, confirmation, telemetry, or application state.
- Moving Refloat configuration, telemetry layouts, controller shutdown, firmware compatibility, SQLite, the loopback server, Bluetooth, location, files, lifecycle, theme, permissions, or UI behavior.
- Adding a general Refloat runtime, generated bindings, callbacks, or Rust-owned lifetime across a future host boundary.
- Changing the frontend contract, stored data, exports, diagnostics, or user-visible behavior.

## Decisions

### Add one focused safe codec crate

Add `backends/core/refloat-control` as workspace package `float-view-refloat-control`. It will forbid unsafe code and have no normal or third-party development dependency. It will remain independent of VESC framing, CAN forwarding, host/application models, compatibility catalogues, and every bridge crate.

A broader `refloat-protocol`, telemetry decoder, or configuration extraction was rejected because those boundaries are substantially larger and include evolving layouts or state. Extracting only the three-byte custom-app-data prefix was rejected as too small to retire meaningful duplicated behavior. The live-control codec is small enough for exhaustive parity while still moving one complete safety-sensitive wire concern.

### Use fixed-capacity payload values and typed profile inputs

The safe API will expose typed lighting profile, control profile, lighting target, and lighting state values. Encoders return one fixed-capacity payload value with an explicit length; the longest supported request is nine bytes. This keeps allocation and future bridge copying bounded without exposing Rust allocator ownership.

Package and flywheel encoders accept booleans. Lighting accepts only `Led` or `Headlight`, or no target for the existing zero-mask query, plus the current two-bit state. Fatal clear accepts the caller-provided control profile and fails with the closed unsupported-command error when unavailable. Compatibility version lookup deliberately remains native because it is policy rather than wire encoding.

Returning framed or CAN-prefixed bytes was rejected because route ownership and framing already have independent components. Returning host-specific errors or models was rejected because representation mapping belongs to later bridge changes.

### Preserve exact Refloat control bytes and closed response validation

Every request begins with VESC custom-app-data command `36` and Refloat package ID `101`. Package uses command `12` and inverted enable byte. Flywheel uses command `22`, mode byte `0x81` or `0x80`, four zero parameters, and footpad-abort byte `1`. Legacy lighting uses command `202` followed by mask and flags; stable lighting uses command `20`, three reserved zero bytes, then mask and flags. Fatal clear uses command `36` and parameter `1` only when the supplied profile permits it.

Lighting encoding preserves the non-target light from current state, updates only the selected target, and emits mask zero without changing flags for a query. Response decoding requires exactly four bytes, the exact prefix, the command selected by the profile, and no flag bits above the two defined lights. It returns only the two booleans. Errors remain closed malformed or unsupported-command variants and must not format payload bytes or state.

### Use existing source-derived evidence without dependencies

Tests will `include_str!` the committed `backends/fixtures/refloat/product-controls.json` and use a tiny test-only named-array extractor, following existing core component practice. The crate will not add JSON, fixture-generation, code-generation, or numeric dependencies.

Tests will cover all committed request/response cases, both lighting profiles, zero-mask queries, all current/target/enabled state combinations, exact payload capacities, unavailable fatal clear, empty and every wrong-length response, wrong command/prefix/profile, undefined flag bits, deterministic output, and redacted errors.

### Keep candidate and production evidence distinct

`just verify` will build, format, lint, and test the candidate and audit the expanded workspace. Existing Swift/Kotlin and canonical tests continue to prove production behavior. Focused `control-direct` physical scenarios on the connected Android/iOS devices and macOS host verify no regression in production Bluetooth, request routing, native codec behavior, state confirmation, and cleanup; they do not claim the candidate executed.

Later Apple and Android changes will introduce and select separate bridges. Each host cutover must exercise these controls through Rust, preserve compatibility/routing/authorization/application behavior, pass clean packaging and physical checks, and delete that host's native codec before claiming ownership transfer.

## Risks / Trade-offs

- **The candidate can drift while native codecs remain live.** → Reuse the same immutable source-derived fixtures, exhaustively cover the tiny state space, audit production call sites, and follow with per-host cutovers before expanding scope.
- **A fixed-capacity payload wrapper is slightly more verbose than a byte vector.** → It makes the nine-byte resource bound explicit and simplifies later caller-owned bridge buffers without an allocator contract.
- **Compatibility policy remains duplicated temporarily.** → Keeping version admission out of the wire codec prevents this increment from combining policy and encoding; migrate it only as its own proven boundary.
- **Physical runs do not execute the candidate.** → Label them regression evidence only; later cutovers must run live controls through their selected Rust bridges before native deletion.

## Migration Plan

1. Add the focused safe crate and update exact workspace, ownership documentation, dependency checks, and production-isolation audits.
2. Implement strict bounded encoding/decoding and source-derived/exhaustive tests.
3. Run focused Rust/native/conformance/audit checks and the complete `just verify` gate.
4. Run focused Refloat control physical scenarios through production BLE on Android, iOS, and macOS.
5. Leave both native codecs selected and follow with one host-cutover OpenSpec change at a time.

Rollback removes the isolated crate and its workspace/audit/documentation entries. There is no production call-path or data migration to reverse.

## Open Questions

None. Bridge representation, host cutover order, compatibility migration, telemetry/configuration migration, and application behavior are deliberately deferred.
