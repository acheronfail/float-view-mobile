## Context

Apple and Android already decode `COMM_FW_VERSION` through the same safe Rust firmware-identity component, but each host still owns an identical VESC compatibility catalogue. That native policy admits controller hardware type `0` at exactly versions 6.02, 6.05, 6.06, and 7.00; stable builds are read/write, nonzero test builds are read-only, and shutdown is available only to stable 6.05, 6.06, and 7.00. The duplicated controller-discovery state machines call this policy throughout route selection and later application mutation gates.

The existing `vesc-firmware-identity` safe crate exposes the complete bounded `FirmwareIdentity` model. Its temporary host records and bridges exist only because Swift and Kotlin currently consume decoded identities. A future unified Rust runtime will call safe crates directly and remove those boundaries.

## Goals / Non-Goals

**Goals:**

- Provide one dependency-minimal, unsafe-free Rust owner for the exact VESC controller compatibility catalogue and access decisions.
- Consume `FirmwareIdentity` directly so later Rust controller discovery and mutation coordination compose safe APIs without serialization or another bridge.
- Match every selected Swift/Kotlin decision for hardware type, version, test marker, read/write operation, and shutdown availability.
- Prove fixture provenance, exhaustive finite-domain behavior, diagnostic privacy, production isolation, complete software verification, and unchanged physical behavior.

**Non-Goals:**

- Selecting the candidate from Swift or Kotlin, deleting native policy, or adding Apple FFI/Android JNI.
- Moving controller discovery, route continuity, scheduling, mutation coordination, Refloat policy, protocol decoding, persistence, loopback serving, or OS capabilities.
- Adding version ranges, inferred compatibility, hardware-name admission, new supported firmware, or a generated API.

## Decisions

### Use the safe identity model directly

The crate will depend only on `float-view-vesc-firmware-identity` and accept `&FirmwareIdentity`. It will not parse payloads, decode the canonical host record, invent a second identity type, or depend on either bridge. This is the API the unified Rust runtime needs and avoids preserving migration serialization internally.

Alternative considered: accept only `(major, minor, test_version, hardware_type)` facts. That is mechanically sufficient today, but it would allow callers to bypass the structurally validated identity owner and creates another compatibility input model to keep synchronized.

### Represent the catalogue and decisions as closed typed values

The crate will expose a small `FirmwareVersion`, `AccessMode`, and `Operation` vocabulary plus a fixed four-entry supported catalogue. `access_mode`, `supports`, `permits`, and `supports_shutdown` will be total functions. Exact version equality and `MOTOR_HARDWARE_TYPE` are the only admission keys; names, UUID, CRC, custom-configuration count, and other provenance cannot broaden policy.

Alternative considered: ranges or capability inference. The canonical product contract explicitly requires exact audited releases, so inference would weaken safety and make fixture evidence non-authoritative.

### Keep the candidate unselected and bridge-free

This change adds only a safe workspace member, focused tests, audits, and documentation. Swift and Kotlin remain production owners, and the bridge catalogues remain exactly ten entries. No compatibility record or host ABI is useful because the next intended consumer is Rust controller discovery, not another native adapter.

Alternative considered: cut over each host through two new compatibility bridges and delete 99 native lines. That would increase the temporary bridge count immediately before the migration phase that is intended to delete those bridges, while providing no production behavior benefit.

### Use exhaustive policy tests plus immutable fixture composition

Focused tests will enumerate the complete major/minor domain, representative stable/nonzero test markers, every hardware-type boundary, both operations, and shutdown decisions. Fixture tests will unwrap the four immutable THOR300 firmware frames through the existing safe frame and identity crates and assert their exact compatibility decisions. Adversarial tests will prove that all unrelated identity fields and private values are ignored and redacted diagnostics cannot expose them.

## Risks / Trade-offs

- **The repository temporarily has three policy implementations.** → Keep Rust unselected, audit production source selection, and make Rust controller discovery the next increment so the candidate becomes a direct dependency rather than an isolated duplicate.
- **A future supported release must update native and Rust catalogues until cutover.** → Exact fixture and source audits make drift visible; do not introduce ranges or code generation for four entries.
- **Direct `FirmwareIdentity` coupling ties compatibility to the structural model.** → This is intentional downward composition; compatibility cannot accept identities that bypass structural validation.
- **Physical verification is broad relative to the small candidate.** → The all-host matrix proves adding the workspace member and audits did not alter packages or selected behavior; only slow mobile location acquisition may be deferred.
