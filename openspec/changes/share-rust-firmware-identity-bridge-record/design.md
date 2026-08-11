## Context

Both production host families now select `vesc-firmware-identity`, but the Android JNI and Apple FFI crates each own a second copy of the same 26-byte header rules, bounded replacement-decoded names, flags, optional CRC representation, record serialization, decoder-error classification, and exact-record tests. That duplication was deliberately accepted while host cutovers were independent; changing an already-selected peer would have broadened each cutover. With both cutovers complete, the bridge crates contain 418 production lines between them and protocol-neutral record work is the largest avoidable part.

The public boundaries are already locked: Apple exposes a two-symbol caller-owned-buffer C ABI and Android exposes a two-symbol primitive-array JNI ABI. Swift and Kotlin independently validate the same existing bytes before constructing their native models. The refactor must preserve those symbols, bytes, status numbers, validation order, limits, artifacts, and adapters while keeping unsafe and host-specific behavior outside the safe component.

## Goals / Non-Goals

**Goals:**

- Make `vesc-firmware-identity` the single source for the existing bounded identity record layout, serialization, constants, and decoder-to-record outcome.
- Keep the safe crate dependency-free and unsafe-free while making both bridges thin consumers of one portable result.
- Delete both bridge-local record encoders and their duplicate exhaustive record fixtures/tests, retaining focused boundary tests for each ABI.
- Preserve exact production behavior and provide a low-risk pattern for later, separate record-sharing changes in other protocol families.

**Non-Goals:**

- Changing any public C/JNI signature, ABI/status number, record byte, capacity, header, archive/library, Swift/Kotlin adapter, or native model.
- Sharing Apple pointer handling with Android JNI, combining bridge crates or artifacts, or adding a general native runtime.
- Introducing `cbindgen`, UniFFI, bindgen, build-time generation, declarative/procedural export macros, a new dependency, a generated source tree, or a committed native artifact.
- Changing VESC structural decoding, compatibility, routing, scheduling, application behavior, persistence, frontend contracts, or any OS-owned capability.
- Refactoring BMS values, controller data, custom configuration, framing, Refloat control, or telemetry in the same change.

## Decisions

### Put a portable record module in the existing safe component

`vesc-firmware-identity` will expose a small safe record API containing the existing ABI/record version constant, payload and record bounds, flags/layout constants needed by consumers, one bounded record value, and a closed record outcome that distinguishes success, unsupported command, structural truncation, and impossible internal representation. The operation will invoke the existing structural decoder and serialize its result once. It will expose no pointer, JNI type, C layout, host status, callback, retained reference, or platform concept.

Adding a seventh per-family `bridge-contract` crate was rejected because it would add package/manifests/build inputs while owning only one component's representation. Making Android depend on the Apple bridge, or the reverse, was rejected because it would invert platform ownership and couple artifacts. Keeping two encoders was rejected because it preserves the maintenance problem this change exists to solve.

### Preserve the byte-record boundary rather than export Rust or C layouts

The canonical value will still publish the exact existing byte record. Rust allocations, references, strings, and layouts will not cross either host boundary. The Apple bridge will continue validating disjoint caller regions before bounded publication; the Android bridge will continue validating the call-local JNI environment, primitive arrays, lengths, and aliases before copying. Each bridge maps the portable outcome to its existing fixed statuses and metadata.

`#[repr(C)]` structures were rejected because identity contains bounded variable-length strings and optional data, and a native layout would introduce padding, lifetime, and ownership contracts without helping JNI. JSON or another serialization dependency was rejected because it would add allocation, parsing, and a new wire format to a hot protocol boundary.

### Prefer ordinary functions over macros or generated bindings

The meaningful duplication is the record contract, not the spelling of two exported functions. Both exported entry points will remain explicit and reviewable. `cbindgen` would affect only the Apple header and cannot generate Kotlin JNI declarations or remove boundary logic. An export macro would reduce visible lines while hiding exact symbols and unsafe call shapes. UniFFI or schema code generation would change the ABI and create a runtime/generated layer for transitional bridges that should eventually disappear behind Rust application behavior.

After record ownership is removed from more than one family, a later exploration may identify truly identical platform call shells suitable for ordinary generic helpers in `apple-ffi-support` or `android-jni-support`. This change will not pre-empt that evidence with a configurable mega-wrapper.

### Consolidate semantic tests while retaining adversarial boundary evidence

The safe component will become the exhaustive owner of exact source-derived records, replacement UTF-8, optional-name/CRC forms, maximum extents, unsupported commands, truncation, and impossible constructed identities. Each bridge retains at least one exact success pass-through plus all platform-specific null, alignment/alias, capacity, exception, status, metadata, and panic-containment tests. C harness, Swift, JVM, packaging, symbol, and production-path checks remain unchanged.

Repository audits will reject `encode_record`, bridge-local flag/layout constants, or parallel record outcome types in either host bridge and will require a material combined production-line reduction. They will also preserve the empty production dependency graph and unsafe prohibition of the safe component.

## Risks / Trade-offs

- **[Risk] A shared record defect affects all three hosts at once.** → Move existing exact vectors before deleting either encoder, compare both compiled ABIs byte-for-byte, retain bridge boundary tests, and run complete software plus physical identity regression.
- **[Risk] Moving error classification changes an established host status.** → Keep public status mapping in each bridge and test every portable outcome through both compiled boundaries.
- **[Risk] The safe protocol component becomes polluted with host concepts.** → Admit only a platform-neutral bounded byte representation and closed semantic outcome; audit out JNI, C exports, unsafe code, callbacks, and platform dependencies.
- **[Risk] Removing duplicate tests weakens boundary coverage.** → Remove only semantic record duplication; retain every distinct Apple pointer and Android VM/reference failure class.
- **[Trade-off] C headers and Swift/Kotlin record readers remain hand-written.** → They are public host contracts and model mappings rather than duplicate Rust record construction; ABI-version and compiled-adapter tests continue detecting drift.

## Migration Plan

1. Add the portable record API and exhaustive exact-record tests to `vesc-firmware-identity` without selecting it from either bridge.
2. Migrate the Apple and Android bridges to the shared outcome one at a time while comparing exact ABI vectors and adversarial behavior.
3. Delete both superseded encoders, duplicate semantic helpers/tests, and bridge-local record ownership; update audits and documentation.
4. Run locked Rust/C/JVM/Swift checks, clean Apple and four-ABI Android packaging, strict OpenSpec validation, and `just verify`.
5. Run the applicable identity, direct/CAN control, BMS, and mobile-background regression matrix on Android, iOS, and macOS against the ESP32 fixture, with only bounded-time location sample absence eligible for the established manual deferral.

Rollback is a normal source revert restoring the bridge-local encoders. No persistent data, frontend contract, public ABI, native model, or application migration is involved.

## Open Questions

None. The current record bytes, limits, statuses, boundary ownership, verification gates, and one-family scope are fixed.
