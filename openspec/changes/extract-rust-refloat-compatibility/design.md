## Context

Both production backends currently implement the same compatibility sequence in native code. They emit unframed Refloat INFO request `[36, 101, 0, 2, 0]`, decode legacy compact identity bytes or the version-2 response, map the reported wire identity through nine audited source-package entries, select one of three telemetry profiles and one of two control profiles, and admit a negotiated dynamic layout only when the same 16 always-present and 10 running-only semantic IDs are present.

The INFO decoders are only 14–15 lines, but extracting them alone would leave the more consequential profile and layout decisions duplicated and would eventually require a temporary bridge for a nearly trivial parser. The matching compatibility files are 88 Swift lines and 67 Kotlin lines and feed controller discovery, live-control selection, supported-state publication, tests, and session metadata. Together these pieces form one pure, deterministic admission boundary; connection generation, request scheduling, route ownership, errors shown to users, and readiness transitions remain application behavior above it.

Pinned Refloat source defines command `0`, package ID `101`, INFO protocol versions 1 and 2, the version-2 package-version offsets, and the dynamic field identifiers. Canonical requirements close the supported catalogue to source releases 1.0.0–1.0.1, 1.1.0–1.1.2, and 1.2.0–1.2.3 and require the legacy wire identity to remain `1.0` without fabricated patch `.0`.

## Goals / Non-Goals

**Goals:**

- Add one dependency-free, unsafe-free Rust component for exact INFO request construction, bounded response identity decoding, exact compatibility lookup, profile selection, and required dynamic-layout admission.
- Model wire versions structurally so legacy `1.0` remains distinct from semantic-version identities with a reported patch byte.
- Preserve the exact audited source-release catalogue, legacy/dynamic/dynamic-alert telemetry families, legacy/stable lighting profiles, fatal-clear availability, and required field sets.
- Reject unknown, prerelease-equivalent, malformed, oversized, duplicate, or incomplete inputs with closed payload-safe failures and no partial result.
- Keep both native production paths, bridge catalogues, packaged artifacts, application coordination, and OS capabilities unchanged.
- Prove candidate behavior with focused source-derived/adversarial tests, `just verify`, and all-host ESP32 regression.

**Non-Goals:**

- Adding Apple FFI or Android JNI, selecting Rust in production, or deleting Swift/Kotlin INFO and compatibility code.
- Moving controller-discovery state, readiness transitions, user-facing issue construction, direct/CAN routing, VESC framing, scheduling, session metadata, or loopback serialization.
- Moving dynamic/legacy telemetry decoding, Refloat control encoding, configuration, VESC compatibility, application behavior, SQLite, local webserver, Bluetooth, location, lifecycle, native files, theme, permissions, or UI behavior.
- Broadening compatibility from the exact catalogue, parsing INFO package names/capabilities for new behavior, or accepting prerelease/range matches.

## Decisions

### Extract one compatibility component rather than an INFO-only decoder

Add workspace package `float-view-refloat-compatibility` at `backends/core/refloat-compatibility`. It will own the complete pure admission calculation: INFO request bytes, response wire identity, exact catalogue lookup, derived telemetry/control profiles, and dynamic-layout required-ID validation.

An INFO-only crate was rejected because its tiny parser would not remove the duplicated decisions that determine whether a controller reaches `ready` and would create an unnecessarily narrow future bridge. Moving `ControllerDiscovery` was rejected because its route probing, generation, timeout, persistence-hint, and application-state behavior is a larger stateful slice that deserves an independent later change. Adding host bridges now was rejected because candidate semantics can be proven without increasing migration scaffolding.

### Use structural versions and closed profile enums

The safe API will represent a wire identity as major/minor plus optional patch. Legacy compact byte `10` produces `1.0` with no patch; compact bytes greater than `10` produce their decimal major/minor form; INFO protocol version `2` requires the complete source-defined version-2 prefix and reads major/minor/patch from offsets 25–27. Formatting remains a mechanical host concern in later cutovers.

The catalogue will use fixed compile-time entries for all nine audited source releases. Exact lookup returns a closed telemetry profile (`legacy-fixed`, `dynamic`, or `dynamic-alerts`) and control profile (legacy or stable lighting plus fatal-clear availability). It will not parse free-form semantic-version strings, infer ranges, or use package names, suffixes, hashes, capabilities, VESC identity, or route data as compatibility keys.

Strings were rejected as the safe model because they allow ambiguous `1.0` versus `1.0.0` handling and push validation into callers. A third-party semantic-version dependency was rejected because the accepted set is closed, reported identities are byte fields, and range matching is prohibited.

### Preserve bounded source response behavior

The candidate will accept complete unframed VESC custom-app-data payloads only, validate `[36, 101, 0]`, enforce the existing 4,096-byte VESC payload bound, and return no partial identity. The exact request operation returns `[36, 101, 0, 2, 0]`; framing and CAN forwarding remain outside the component.

Legacy compact forms require the version byte and preserve the current bounded tail-tolerant behavior because source protocol-1 responses carry build/LED bytes while canonical fixtures deliberately exercise the minimal identity prefix. Version-2 requires at least the complete 61-byte source-defined response prefix and ignores bounded metadata/capability bytes that are not compatibility keys. Unsupported protocol bytes fail closed. Payload bytes and private package metadata never enter errors or diagnostics.

Requiring one exact legacy length was rejected because it would regress the canonical minimal 1.0 identity fixture or the source response with its legacy tail. Parsing unused package names, hashes, suffixes, tick rates, or capabilities was rejected because it would expand scope without changing current admission behavior.

### Admit dynamic layouts through bounded required-ID checks

The component will expose the exact 16 always-present and 10 running-only required identifiers and a layout-admission operation over two caller-owned string slices. It will enforce at most 64 total identifiers, at most 64 UTF-8 bytes per nonempty identifier, at most 4,096 aggregate bytes, and global uniqueness before checking that every required identifier occurs in the correct group. Order and additional bounded future identifiers remain acceptable.

Depending on `refloat-dynamic-telemetry` was rejected for this candidate because compatibility policy needs only validated names and keeping an empty production dependency graph makes the boundary reusable during independent host cutovers. Duplicating dynamic value decoding or layout record serialization was also rejected; later unified Rust orchestration may connect these ordinary APIs without another host boundary.

### Keep candidate and production evidence distinct

Focused Rust tests will cover source and canonical INFO shapes, all catalogue entries, every profile, required-field admission, malformed envelopes/extents/versions/names/bounds, exact boundary values, deterministic outputs, and redacted errors. Repository audits will prove the crate has no bridge symbol, build-catalogue entry, native package dependency, generated binding, or production call site.

`just verify` will test the candidate while native tests continue proving the selected Swift/Kotlin behavior. Physical Android, iOS, and macOS runs exercise the existing production paths only and therefore prove regression freedom, not Rust selection. Separate Apple and Android cutovers must later add their temporary host mapping, prove physical INFO/profile selection through Rust, and delete native ownership independently.

## Risks / Trade-offs

- **The catalogue can drift while native code remains selected.** → Test all three implementations against the same exact releases, profiles, required IDs, and canonical fixtures, then follow with separate host cutovers.
- **Combining wire parsing and policy could become an application grab-bag.** → Limit the crate to deterministic INFO identity and profile/layout admission; keep discovery effects, routes, timeouts, state, persistence, and presentation outside it.
- **Tail-tolerant INFO parsing accepts bytes the component does not interpret.** → Retain the enclosing 4,096-byte frame bound, require the complete relevant prefix, ignore metadata as a compatibility key, and expose no payload-bearing diagnostics.
- **Required identifier lists repeat names also understood by dynamic telemetry.** → Keep one compatibility policy owner now; consolidate ordinary Rust models only when a later unified runtime owns orchestration, rather than coupling an unselected candidate to a protocol decoder prematurely.
- **Physical runs cannot execute an unselected crate.** → Label them regression evidence only and require selected-path physical proof in each later host-cutover change.

## Migration Plan

1. Add the isolated safe crate, workspace/documentation entry, and exact dependency/ownership/production-isolation audits.
2. Implement INFO request/identity, structural versions, the exact compatibility catalogue, profile decisions, and bounded layout admission.
3. Add source-derived and adversarial tests, then run focused checks and `just verify`.
4. Run production-path physical regression on Android, iOS, and macOS with the ESP32 fixture.
5. Leave Swift and Kotlin selected; follow with independent Apple and Android cutovers.

Rollback removes the isolated crate and its workspace/audit/documentation entries. No native artifact, persistent state, contract, or user-visible behavior requires migration or reversal.

## Open Questions

None. Host record shape, cutover order, controller-discovery extraction, INFO capability expansion, and unified-runtime integration remain deliberate later decisions.
