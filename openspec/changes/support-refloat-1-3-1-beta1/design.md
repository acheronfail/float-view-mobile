## Context

See proposal.md for motivation and capability scope. Before this change, the shared Rust compatibility owner decoded bounded INFO suffixes but rejected every nonempty suffix before stable compatibility lookup. The telemetry service already retrieves bounded command-35 firmware-fault history; its codec, service records, and SQLite validation assume alert ID 1 and `active == (code != 0)`. Some canonical specifications still describe the older command-35 exclusion. This change replaces that exclusion only where beta support requires it and preserves older production behavior.

The upstream `v1.3.0...v1.3.1-beta1` diff retains INFO v2, command-33 telemetry and the product-control wire family. It adds stop condition 7 and alert ID 2 (`ATYPE_ERROR`), uses code 0 for timeout activation as well as clearing, and changes full configuration metadata. The beta's timeout error is independent of `ALERT_FW_FAULT`, which alone supplies the firmware fault code and fatal semantics. Command 33 supplies the stop condition and generic error reason but does not identify the active timeout alert; command 35 supplies the active mask and retained transitions.

Source references: [beta tag](https://github.com/lukash/refloat/tree/v1.3.1-beta1), [comparison](https://github.com/lukash/refloat/compare/v1.3.0...v1.3.1-beta1), and the vendored 1.3.0 `src/main.c`, `src/state.h`, `src/alert_tracker.h` and `src/conf/settings.xml`. Implementation must resolve and record the beta tag's complete immutable commit before generating evidence; release notes alone do not authorize a protocol profile.

## Goals / Non-Goals

**Goals:** Extend existing closed profiles and bounded owners; distinguish active timeout, retained stop reason, recovered timeout events, firmware faults and fatal latch; carry these semantics through both native hosts without frontend protocol ownership.

**Non-Goals:** No Tune conversion, cross-version Tune compatibility, new runtime-tuning command, generic alert subsystem, speculative future release support, extra transport lane, or new dependency. No change to OS location ownership, firmware balancing behavior, supported-data reset policy or archive numbering solely for a firmware version.

## Decisions

### Preserve the complete exact release identity

Reuse the bounded INFO identity parser and admit exactly numeric 1.3.1 plus suffix `beta1`. Stable entries continue to require an empty suffix. Extend the existing closed compatibility/catalogue representation as needed so suffix identity survives profile selection, canonical records, configuration keys, public supported-version guidance and Tune comparison. Audit all same-build C/JNI consumers if canonical records change; do not create another identity parser or parallel native catalogue.

Source and wire versions carry one validated suffix value with the wire's fixed
20-byte zero-padded representation. Empty suffix means stable. INFO display and
admission share that identity; do not retain a second suffix string or a
release-specific boolean/constructor. Generic suffix representation does not
broaden admission: only a complete identity in the audited catalogue is eligible.
IMU timeout support is an explicit property of each compatibility entry, never
inferred from suffix text. Text lookup uses the same catalogue.

The internal canonical compatibility record advances to version 2, retains its
12-byte header (byte 10 now carries the IMU-timeout capability), and appends the
20 suffix bytes. The parser validates suffix padding, exact catalogue identity
and capabilities together. This same-build Rust record has no external native
or persisted consumers; it is independent of storage baseline and archive
numbering. Reject old/internal malformed records rather than adding a fallback. Activation effects box their one-time facts so the larger complete version identity does not enlarge every coordinator effect.

A numeric 1.3.1 allowlist entry alone is rejected because it could admit a later beta or stable release. The build hash is provenance evidence, not an additional private identifier published in snapshots. Existing malformed-input bounds remain unchanged.

### Reuse 1.3 telemetry and controls

Select the existing command-33 fast/slow/GNSS masks and stable control profile after a source audit. Retain the selected route, one scheduler, existing adaptive cadence, low-cadence history read and generation checks. Do not introduce a duplicate command-10 fault polling lane where the current selected profile already uses command 35. Preserve older profiles exactly, including their current production history behavior.

Treat schema metadata as identity even if serialization is unchanged. Pin the vendor submodule to the beta tag during implementation, retaining old source tags and immutable supported fixtures. Extend the existing configuration generator and catalogue, compute the complete beta fingerprint and signature, and verify them against source-derived fixtures. Reusing the stable fingerprint or dropping the beta suffix is rejected. Saved Tunes remain isolated by exact version, fingerprint and signature; no 1.2 conversion UI or conversion arithmetic is included.

### Decode two explicit alert kinds

Extend the existing bounded alert record with a closed firmware-fault/IMU-timeout kind. For ID 1 retain existing nonzero activation code, zero clear code and bounded fault-name parsing. For ID 2 require code 0 for both states and interpret `active` independently. Admit active mask bits 0 and 1 for the beta and require the extension word to remain zero. Unsupported IDs/masks and inconsistent shapes fail the complete response atomically. Older profiles do not gain timeout support from a numeric version comparison.

The command-35 active mask is authoritative for `imuTimeoutActive`. Stop code 7 maps independently to `imuTimeout`; a retained stop cause does not prove that an error is still active. Preserve the generic source alert reason without turning it into a fabricated VESC firmware fault. A new generation starts with nullable timeout state until a valid current-generation mask arrives. A timeout or malformed response cannot clear a previously observed active timeout; normal freshness rules continue to govern mutation authority.

Reject mapping timeout to `FirmwareFault::Unknown`: doing so would mislabel a package error and lose valid activation records with code 0. Reject inferring active state from stop condition: the firmware can retain a stop reason after recovery.

### Extend existing bounded history without changing old events

Retain the history cursor, source-clock mapping, overlapping-read deduplication and 20-record response limit. Include alert kind in deduplication so simultaneous records from different kinds cannot collapse. Maintain wire order for same-tick transitions; preserve observation time separately from nullable approximate occurrence time. Reset cursor, deduplication and active state on route/generation replacement. Existing clock-wrap and unavailable-clock behavior remains bounded and never fabricates an exact occurrence time.

Add `imuTimeoutHistory` events with details `{active, sourceTicks, occurredElapsedMs}` to the existing extensible event storage and paging/export path. Sampled `stopCondition` transitions remain separate observations. Keep `firmwareFaultHistory`, `lastFirmwareFault` and the 20-entry firmware-only live list unchanged. Reuse the existing 10,000-record recovered-history budget across firmware-fault and timeout events per session, rather than creating an unbounded second budget. Preserve existing atomic commit markers and storage-full behavior. Guest recovery updates only ephemeral state.

Add `LiveSample.imuTimeoutActive: boolean | null` and the `imuTimeout` stop-condition literal to the hand-written contract and every projection/client/mock validator. Historical timeout rows use the existing event observation fields and sequence. Frontend live and historical labels distinguish timeout activation, clearing and retained stop reason; simultaneous firmware faults remain visible. Native hosts continue executing OS effects through their existing bindings; product projection remains shared.

### Apply error-aware mutation gates

An authoritative active timeout blocks package enablement changes, flywheel enablement, configuration writes and Tune application at the shared mutation boundary. Keep the existing guarded flywheel-disable escape. The timeout neither fabricates a fatal latch nor independently enables command 36. An unrelated firmware-fault latch still follows its existing clear gates. Timeout clearing does not bypass ordinary access, generation, freshness, mode, current-revision and verified-readback requirements.

### Preserve supported application data

Use additive enum/event semantics in the existing extensible storage if supported readers and validators can accept them without a schema-layout change. Update session readers, integrity validation and archive import/export together; prove old supported events, Tunes and baseline archives still round trip. No schema change is assumed merely because a new firmware release is admitted.

If implementation discovers an actual incompatible supported format transition, update this design before proceeding, increment the appropriate storage marker and implement a bounded crash-safe tested upgrade. Never reset supported data, alter frozen baseline fixtures, reuse pre-baseline fallback readers, or couple archive numbering to package versions.

## Risks / Trade-offs

- [A beta shares numeric identity with future releases] → Exact suffix-aware admission and rejection vectors for stable 1.3.1, beta, beta2 and malformed suffixes.
- [Firmware-only record assumptions exist in several consumers] → Update codec, service, deduplication, writer, readers, shared projection and tests as one typed path; keep old firmware events unchanged.
- [History is bounded and can miss events before retrieval] → Retain existing source buffer, cadence and capacity limits; describe recovered history honestly and keep occurrence timing nullable.
- [Polling observes timeout at low cadence] → Use the existing bounded history schedule; preserve observed active errors until authoritative clearing and rely on current freshness gates without adding fast history polling.
- [New enum/event values reach persisted/exported data] → Test supported baseline imports, integrity checks, reopen and round trips; add an explicit upgrade only for a demonstrated incompatible transition.
- [Real IMU failure is difficult to reproduce safely] → Exercise activation/clear and adversarial cases in source-derived simulation; perform targeted beta interoperability on real hardware and record real fault coverage separately without requiring a hazardous induced failure.

## Migration Plan

Complete and validate planning before application changes. Follow the repository's planning/implementation/archive phases when commits are authorized; implementation is now authorized, while commits still require explicit authorization. Implement source fixtures and exact identity first, then typed telemetry/history and storage, followed by contract/UI and both host integration checks.

No user data is converted automatically. Rollback must preserve newly written timeout records and supported baseline data; disabling beta admission is acceptable, deleting beta history or assuming an older reader understands new records is not. Keep the change unarchived until relevant implementation checks pass and any required hardware coverage is recorded honestly.

### Review corrections and beta retirement

Guarded mutations on the beta require positively confirmed inactive IMU timeout state; an unavailable first command-35 observation cannot authorize a write. Older profiles without this capability retain their existing gates. Live snapshots expose a separate bounded `imuTimeoutHistory` collection (20 records), including recovered clear records and observed activation/clear transitions. Initial inactive state alone is not history. Firmware-only history remains separate. The history menu is discoverable only with an active firmware fault or timeout, a fatal latch, or actual firmware/IMU history. Persisted sampled transitions remain reviewable and preserve timeout-only rides.

When stable 1.3.1 is released and audited, replace beta1 admission, implementation naming and source fixtures with stable 1.3.1 support rather than retaining both live firmware profiles. This does not authorize deletion of supported stored histories or Tunes, nor automatic Tune conversion. Until that follow-up, stable 1.3.1 remains unsupported.

### Alert freshness and historical reconciliation corrections

Track the last successful current-generation command-35 observation on the monotonic clock independently of fast telemetry. Inactive authority lasts at most the larger of configured idle cadence and history cadence plus two request timeout budgets (slow/history serialization); a failed history read invalidates it immediately. Guarded package, flywheel-enable and configuration/Tune requests recheck this authority at submission, even before another sample publishes. Active warnings persist until a valid clear; expiry projects unknown rather than fabricating clearance. Older profiles and flywheel disable retain their gates.

Retained-stop fallback rows carry null occurrence time and a separate observation time. Reconcile sampled timeout transitions one-to-one and in order against recovered records within the interval between the previous state observation and the sampled transition. Delayed retrieval does not impose a fixed two-second occurrence tolerance. Unknown source times stay unknown; use ordered observation evidence conservatively, and preserve recovered records and distinct episodes.
