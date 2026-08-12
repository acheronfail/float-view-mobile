## Context

`refloat-legacy-telemetry` is a dependency-free, unsafe-free Rust candidate that already decodes every fixed Refloat command-10 detail level and active-firmware-fault form. Apple production still performs the same length checks, byte reads, scaling, semantic-state mapping, and fault classification in `RefloatLegacy.swift`; iOS and macOS share that source. Existing Apple protocol candidates are selected through independent caller-owned-record C bridges built from source by one allowlisted helper.

The cutover must preserve the existing `RefloatLegacyTelemetryDecoder.decode` API and `RefloatTelemetryRecord`, keep Android on its Kotlin decoder, build offline for every Apple target family, expose no Rust allocation/layout/lifetime, and leave compatibility, polling, routing, state, persistence, transport, and OS capabilities native. The bridge is explicitly transitional and will be deleted when the complete application runtime moves to Rust.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole shared Apple production owner of fixed Refloat command-10 structural and semantic decoding.
- Put the canonical versioned result record in the safe crate so the later Android bridge can consume it without another encoder.
- Add one temporary two-symbol Apple C bridge with fixed statuses, caller-owned buffers, checked disjoint regions, and panic containment.
- Preserve normalized telemetry models, established errors and all higher-level Apple behavior while deleting superseded Swift wire algorithms.
- Prove exact records, adversarial boundaries, Apple packaging/source selection, Android isolation, complete software behavior, and all-host physical regression.

**Non-Goals:**

- Selecting Rust legacy telemetry on Android.
- Moving dynamic Refloat telemetry, compatibility, request construction, direct/CAN routing, framing, scheduling, polling, cache merging, application behavior, SQLite, loopback transport, BLE, location, background work, files, theme, lifecycle, permissions, WebViews, or the frontend contract.
- Introducing the final Rust application runtime, callbacks, opaque handles, generated bindings, `cbindgen`, UniFFI, macros, committed native binaries, or a new dependency.
- Generalizing or merging existing per-protocol bridge crates; all such bridges remain temporary migration scaffolding.

## Decisions

### Put the result record in the safe decoder crate

Extend `refloat-legacy-telemetry` with an allocation-free `LegacyTelemetryRecord` and closed `LegacyTelemetryRecordOutcome`. `decode_record` delegates to the existing typed decoder and is the only serializer. It preserves unsupported-envelope, truncation, invalid-payload, and impossible internal-result distinctions without exposing payloads in diagnostics. The safe crate remains dependency-free and forbids unsafe code.

The version-1 record has a 16-byte header. It contains version, result kind, detail level, package state, package mode, footpad state, stop condition, setpoint adjustment, compact boolean flags, alert classification, firmware-fault classification/raw code, option flags, and reserved zero bytes. A sample appends the 31 existing numeric fields as finite big-endian IEEE-754 `f64` values in model order for an exact 264-byte record; cleared option flags require canonical zero bits. An active fault is an exact 16-byte record with every irrelevant field zero. Option flags distinguish unavailable controller/motor temperatures, field weakening, distance, odometer, energy counters, battery level, and charging values from numeric zero.

Keeping serialization in the bridge was rejected because Android would later duplicate it. Returning a C struct or Rust allocation was rejected because it exposes layout or lifetime obligations. Text/JSON was rejected because it increases parsing, allocation, and malformed-output surface at this narrow boundary.

### Use a separate temporary two-symbol Apple bridge

Add `refloat-legacy-telemetry-apple-ffi` with exactly an ABI-version query and decode operation. Decode accepts one bounded borrowed payload plus caller-owned record, record-length, and unsupported-command outputs. It maps fixed success, invalid-argument, unsupported-envelope, truncated-payload, invalid-payload, insufficient-output, and contained-internal-failure statuses. `apple-ffi-support` owns mechanical region validation, borrowing, publication, and panic containment; the safe telemetry crate owns every protocol and record decision.

Adding command-10 to another bridge was rejected because it couples unrelated release surfaces. A generated binding or export macro was rejected because two explicit symbols remain easier to audit and the bridge will ultimately be removed.

### Keep Apple mapping strict and mechanical

`RefloatLegacyTelemetryDecoder` becomes a bridge-backed adapter. It validates the linked ABI, exact record extent, version, result kind, closed enum codes, flags, reserved and irrelevant bytes, option/detail consistency, finite numeric values, canonical absent fields, nonzero active-fault code, and complete cursor consumption before constructing the existing `RefloatTelemetryRecord`. Ride state and the active-fault alert are derived only from the already classified record kind/package state, not from command-10 bytes.

After selection, the Swift input-length table, signed/unsigned readers, float decoding, state/mode/footpad/stop/setpoint/alert/fault interpretation, scales, field-weakening sentinel handling, and fallback path are deleted. Representation switches over the stable record codes remain permitted and are covered by rejection tests.

### Extend Apple-only build metadata without changing Android's bridge set

Add an Apple-capable `refloat-legacy-telemetry` catalog entry and make the Android builder reject that entry as unavailable. SwiftPM and Xcode receive the new C module, source inputs, archive search path, and link setting for macOS, iOS device, and iOS simulator. Android's typed six-entry Gradle catalog, JNI tasks, artifacts, packages, and sources remain byte-for-byte independent of the Apple-only entry.

This temporary Apple/Android catalog asymmetry is intentional until the later Android cutover. Creating an unused Android crate or Gradle entry was rejected because it would pretend production integration and widen this change.

### Use focused and physical evidence before deleting Swift logic

Focused safe-Rust tests cover canonical records for detail levels 1–4, active faults, every enum and option combination, sentinel field weakening, exact bounds, non-finite rejection, impossible internal values, and redacted diagnostics. Compiled C tests cover both symbols, statuses, region failures, capacity behavior, panic containment, and exact record bytes. Swift tests cover exact model mapping plus malformed-record rejection and production call selection.

After clean Apple packaging and `just verify`, run identity, direct/CAN control, applicable BMS, and mobile background/session scenarios on Android, iOS, and macOS against the ESP32 fixture. Apple runs exercise production Rust command-10 decoding; Android proves isolation. Only absence of a location sample within its bounded dwell is deferred.

## Risks / Trade-offs

- **[Risk] A large fixed record can drift from the normalized model.** → One safe serializer, exact length/order constants, exhaustive record tests, strict Swift cursor/flag validation, and source-ownership audits make drift fail closed.
- **[Risk] Apple-only catalog metadata can accidentally enter Android builds.** → Keep Android's typed catalogue at six entries, make its builder reject missing JNI metadata, and audit Android packages and task names unchanged.
- **[Risk] Another bridge temporarily increases build surface.** → Keep exactly two symbols, reuse audited support/build tooling, add no dependency or generation, and document planned deletion with the eventual Rust runtime.
- **[Trade-off] Apple and Android temporarily select different decoders.** → Preserve the same normalized API and fixtures, run identical physical scenarios, and make Android its own reversible subsequent OpenSpec change.

## Migration Plan

1. Add the canonical safe record/outcome, focused tests, Apple bridge, C module/harness, catalog metadata, and audits without selecting it from Swift.
2. Wire SwiftPM and Xcode for all Apple target families, then add the strict adapter and focused mapping/failure tests.
3. Select the adapter, delete Swift command-10 wire algorithms, and prove Android isolation, clean packaging, strict OpenSpec validation, and `just verify`.
4. Run focused ESP32 regression on connected Android, iOS, and macOS, deferring only bounded location-sample absence.
5. Commit implementation, sync the three delta requirements, and archive the change.

Rollback is a source revert of the coherent implementation commit. There is no data migration, persisted-format change, dual runtime path, or remote deployment.

## Open Questions

None. Android production selection and the later unified Rust application/runtime boundary remain separate changes.
