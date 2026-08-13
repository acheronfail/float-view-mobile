## Context

`refloat-compatibility` is an unsafe-free Rust component that owns the exact INFO request, bounded legacy/version-2 identity decoding, the closed nine-release telemetry/control catalogue, a canonical version-1 compatibility record, and admission of the canonical dynamic-telemetry layout record. Apple production already consumes those decisions through a four-symbol C bridge. Android still constructs INFO from Kotlin constants, decodes payload offsets in `RefloatInfo.kt`, derives profiles and supported releases in `RefloatCompatibility.kt`, and admits layouts using duplicated required-name lists and set membership.

Android already selects eight independent primitive-array JNI bridges through one locked shell catalogue and typed Gradle registration. Its dynamic-telemetry decoder retains the canonical layout record for the current connection generation but exposes only field lists to higher layers. The compatibility cutover must reuse that record, preserve the existing discovery/control/state behavior, keep Apple unchanged, package offline for the host JVM and every Android ABI, expose no Rust allocation or retained JVM reference, and leave every OS capability and application owner in Kotlin. Both compatibility bridges are transitional and will be deleted when complete application orchestration moves behind one Rust runtime boundary.

## Goals / Non-Goals

**Goals:**

- Make safe Rust the sole Android production owner of INFO request construction, INFO identity decoding, exact release/profile selection, and required dynamic-layout admission.
- Reuse the exact canonical compatibility and layout records already owned by safe Rust and consumed by Apple.
- Add one temporary four-symbol Android JNI bridge with fixed statuses, JVM-owned primitive arrays, bounded call-local copies, deterministic publication, and panic containment.
- Carry one accepted compatibility decision through existing protocol messages, controller discovery, controls, and supported-state publication while preserving observable behavior.
- Delete the superseded Kotlin offsets, constants, catalogue/profile algorithms, required identifiers, set admission, and runtime fallbacks.
- Prove adversarial JNI/record behavior, host and four-ABI packaging, Apple isolation, complete software behavior, and all-host physical regression.

**Non-Goals:**

- Changing the safe compatibility component, canonical record layouts, or selected Apple C ABI and adapter.
- Moving controller discovery effects, request scheduling, direct/CAN routing, framing, telemetry decoding, polling, controls, application behavior, SQLite, loopback transport, Bluetooth, background work, location, lifecycle, permissions, native files, theme, WebViews, or the frontend contract.
- Introducing the final Rust application runtime, callbacks, opaque handles, generated bindings, macros, committed native binaries, or a new dependency.
- Generalizing or merging existing per-protocol bridge crates; they remain temporary migration scaffolding.

## Decisions

### Publish the existing safe decisions through an independent JNI bridge

Add `refloat-compatibility-android-jni` with exactly an ABI-version query, INFO-request publication, INFO decode, and layout admission operation. INFO request and decode use bounded JVM byte-array outputs plus a fixed integer result array carrying produced or required length. Layout admission accepts the canonical layout record and publishes only a fixed admitted/not-admitted result. All calls validate the environment, pending exceptions, references, array shapes, distinctness, capacities, and bounded call-local copies before mapping fixed statuses. `android-jni-support` owns mechanical JNI safety; the safe compatibility crate owns every protocol, catalogue, record, and admission decision.

Adding these operations to another JNI library was rejected because it couples unrelated release surfaces. Reimplementing records or catalogue logic in the JNI crate was rejected because the safe component is already their canonical owner. Returning JVM objects, strings, JSON, direct buffers, or generated bindings was rejected because primitive arrays keep this temporary boundary explicit, bounded, and consistent with existing host tests.

### Mirror Apple’s strict record adapter and decision propagation

The Kotlin adapter validates linked ABI, exact record extent, version/kind, patch-presence rules, closed profile/family/lighting combinations, fatal-clear agreement, reserved bytes, and complete canonical form before constructing the existing Kotlin models plus one `RefloatCompatibilityDecision`. Android protocol messages carry that accepted decision rather than a version string. Controller discovery stores the decision’s wire version, family, and control profile without consulting a Kotlin catalogue; application and loopback consumers use that accepted state.

Keeping string lookup as a second production decision was rejected because it preserves duplicate catalogue ownership. Exposing raw codes above the protocol adapter was rejected because it would spread boundary validation into application code.

### Reuse the current-generation canonical layout record

After dynamic layout decoding, `VescProtocolService` submits the exact retained canonical layout record to Rust compatibility admission before publishing the existing field lists and a supported flag. A rejected layout clears or withholds realtime selection for that generation exactly as Apple does; reset and valid replacement remain owned by the existing Android decoder lifetime. No field-list reconstruction, second serialization, Rust handle, callback, or cross-call JNI reference is introduced.

Passing field lists back across JNI or retaining Kotlin required-name constants was rejected because either creates a second representation or leaves policy duplicated. Moving layout lifetime into Rust was rejected because connection generations and reset remain higher-level Android concerns in this slice.

### Extend the typed Android bridge catalogue by one entry

Complete the existing `refloat-compatibility` shell-catalogue row with the Android package, library stem, JNI prefix, and exact four-symbol allowlist. Add a ninth typed Gradle bridge specification whose Debug/Release outputs feed every matching merge consumer and whose host artifact reaches JVM tests through one explicit property. Dependency inputs include the safe compatibility and dynamic-layout crates, JNI crate, shared JNI support, workspace lock/toolchain, and shared build metadata.

Apple’s package, archive, C symbols, module, SwiftPM/Xcode inputs, and production call remain unchanged. Platform bridge code is not consolidated because C pointer-region publication and JNI primitive-array publication have distinct safety contracts and both boundaries are scheduled for deletion.

### Require deletion and physical selection evidence

Focused Rust tests cover exact safe records, status mapping, null/exception/reference/array/capacity/distinctness failures, initialized outputs, and contained panics. Host JVM tests cover ABI loading, exact requests, all source releases/profiles, strict record mapping, layout admission, discovery/control propagation, established errors, and exhaustive malformed-record rejection. Source and symbol audits prove that Android no longer interprets INFO bytes or owns the catalogue/required sets and that the JNI library exports exactly four bridge symbols.

After clean Android host and four-ABI Debug/Release packaging plus Apple packaging and `just verify`, run applicable identity, direct/CAN control, BMS, dynamic/legacy telemetry, ride, unexpected-disconnect, and mobile background/session scenarios on Android, iOS, and macOS against the ESP32 fixture. Android exercises the newly selected JNI path; Apple proves isolation through its unchanged selected C path. Only absence of a location sample within its bounded dwell is deferred.

## Risks / Trade-offs

- **[Risk] Kotlin record parsing can accept an impossible compatibility combination.** → Mirror the closed Apple validation matrix, require exact extents/reserved bytes, and exhaust malformed combinations in JVM tests.
- **[Risk] Layout admission can use a stale or reconstructed representation.** → Pass only the exact canonical record retained by the current-generation dynamic decoder, clear it on reset, and test replacement and failure behavior.
- **[Risk] A ninth JNI artifact can drift from shell and Gradle catalogues.** → Derive every task from one typed entry and audit exact task names, inputs, outputs, package, symbols, merge consumers, and Apple rejection.
- **[Risk] The cutover may accidentally alter discovery or control behavior.** → Carry the complete accepted decision, preserve higher-level APIs/effects, run canonical software conformance, and exercise production BLE on all three hosts.
- **[Trade-off] Another bridge temporarily increases source and packaging surface.** → Keep exactly four symbols, reuse audited support/build tooling and canonical records, delete the larger Kotlin policy implementation, and require removal with the unified Rust runtime.

## Migration Plan

1. Add the Android JNI crate, exact catalogue/Gradle metadata, focused boundary tests, and audits without selecting it from Kotlin.
2. Add the strict Kotlin record adapter and host-JVM tests, carry the complete decision through existing owners, reuse the canonical layout record, then select Rust and delete superseded Kotlin algorithms and fallbacks.
3. Prove host and four-ABI Android packaging, Apple isolation, strict OpenSpec validation, and the complete `just verify` gate.
4. Run focused ESP32 regression on connected Android, iOS, and macOS, deferring only bounded location-sample absence.
5. Commit implementation, sync the three delta requirements, and archive the change.

Rollback is a source revert of the coherent implementation commit. There is no data migration, persisted-format change, dual runtime path, or remote deployment.

## Open Questions

None. The next higher-level Rust ownership transfer and the later unified Rust application/runtime boundary remain separate changes.
