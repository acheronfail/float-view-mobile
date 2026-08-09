## Why

The previous Rust rewrite combined toolchains, FFI, platform packaging, protocols, application behavior, SQLite, and the loopback server into one cutover, making regressions and ownership failures difficult to isolate. The migration should restart with the smallest source-faithful protocol primitive so Rust behavior and repository integration can be proven independently of native runtime integration.

## What Changes

- Add a minimal locked Rust workspace under `backends/core` containing only one dependency-free VESC framing crate.
- Implement the existing bounded CRC-16, short/long frame encoding, exact frame decoding, and fragmented stream reassembly behavior from authoritative offline VESC sources and current product bounds.
- Verify Rust against immutable source-derived vectors and the same valid, malformed, fragmented, noisy, multiple-frame, CRC, and size-bound cases exercised by the current Swift, Kotlin, and conformance implementations.
- Add deterministic Rust formatting, linting, and tests to the normal `just verify` software gate.
- Establish explicit staged-migration rules: a Rust candidate may coexist with shipped native owners only for a named bounded slice; production ownership changes and native deletion require later host-cutover changes with their own verification.
- Retain the Swift and Kotlin VESC framing implementations as the only production paths in this change. They are scheduled for deletion only after separate Apple and Android cutover changes pass their complete gates.
- Non-goals: adding FFI, JNI, `cbindgen`, an Android NDK build, Apple native-library packaging, a Rust runtime/actor, host callbacks, BLE integration, application behavior, SQLite, the local server, empty future crates, or any frontend-contract or user-visible change.

## Capabilities

### New Capabilities

- `native-core-migration`: Defines the bounded, evidence-first process for incrementally extracting backend behavior into Rust without treating an unintegrated candidate as shipped production ownership.

### Modified Capabilities

- `platform-hosts`: Permit the focused Rust core workspace and staged coexistence while preserving platform ownership of BLE, location, lifecycle, permissions, files, theme, and UI.
- `backend-conformance`: Add Rust component parity checks to the fast software gate without claiming Rust as a supported backend target.

## Impact

This affects `AGENTS.md`, a new `backends/core` Rust workspace, the root toolchain/lock configuration, `justfile` verification, backend ownership documentation, and OpenSpec requirements. The initial production crate uses no third-party dependencies; test-only fixture loading must remain locked and minimal if needed. There is no cross-platform runtime API, FFI, persistence, database, packaged native-library, wire-contract, or user-visible behavior change, and no production source is removed in this step.
