# Design

## Context

Beta3 already shares beta2's telemetry/control profiles and verified identical fixture bytes. Admission still uses an exact release catalogue; configuration service independently holds 12 exact-version/full-fingerprint rows, and the frontend repeats riding-Tune version eligibility. These copies turn a packaging change into unnecessary maintenance and make retiring betas remove access from otherwise compatible boards.

The offline upstream sources provide a useful boundary. `vendor/refloat/doc/commands/INFO.md` describes independently versioned INFO; its current capability bits describe hardware/recorder presence, not operation compatibility. Public `REALTIME_DATA` command 33 is documented in `vendor/refloat/doc/commands/`, whereas commands 31/32 are internal and commands 35/36 are explicitly unstable. A correctly shaped response establishes a usable read exchange under documented semantics, not proof that future writes or controls have unchanged meaning.

## Goals and limits

Permit compatible unknown Refloat versions to connect, display observations and record rides without a new app release. Preserve existing audited functionality, including betas. Keep one backend compatibility decision, shared protocol/schema profiles and small public availability facts. Acknowledge that an unknown release can change units or behavior without changing packet shape; limited support is a bounded compatibility attempt, not certification of all future firmware.

Do not broaden VESC admission, invent capability negotiation, probe undocumented commands, authorize unknown-firmware mutations or add a schema migration framework. An explicit upstream operation/API compatibility contract could later authorize more features independently of release labels; it is neither assumed nor required here.

## Decisions

### 1. One compatibility owner, two levels of usable support

Extend the existing compatibility owner rather than introducing another registry. Its generation-scoped decision contains a read profile and optional verified mutation/control semantics. Known releases select existing audited profiles. A valid unknown Refloat identity can select a candidate public read profile, promoted to limited support only after the probe below. Do not attach a fake stable control profile and rely on frontend flags to make it safe.

Keep current VESC 6.02/6.05/6.06/7.00 identity, route, test-build and hardware checks. Unknown VESC versions remain blocked before fallback. Existing legacy INFO `1.0` and internal 31/32 protocols remain available only through audited mappings. For the fallback require the exact bounded package name `Refloat`, recognized INFO v2 layout, valid echoed flags, supported tick-rate interpretation and complete bounded version/suffix. Unknown capability bits in the fixed INFO v2 bitfield grant no permission and do not by themselves invalidate the known read interface; unknown response layouts or request-flag encodings still fail. Preserve observed identity, including suffix, using existing bounded storage forms; never relabel beta4 or 1.4.0 as beta3. Existing audited numeric patch-suffix handling is not permission to strip arbitrary suffixes.

Use the existing serialized scheduler and verified direct/CAN route. Make one bounded command-33 core probe with the documented float32 flag and a fixed known mask for state, speed and pack voltage, using the existing request timeout and generation cancellation. Require exact echoed flags/masks, expected extent, valid tick interpretation and finite numeric values. No scan across command IDs or encodings, parallel discovery lane or retry loop. No `ready`, saved session activation or durable samples before a usable response. A valid response establishes limited support; failed identification/probe uses existing actionable failure.

### 2. Reads degrade by feature; unavailable status stays unknown

Core polling continues through the proven public profile. Optional command-33 numeric groups can be requested with fixed known masks and exact validation; a failing optional group is disabled for that generation without revoking the core. Use two optional groups: remaining documented numeric mask-1 fields (bits 7–11 and 13–30), and non-GNSS slow mask-2 fields (bits 0–8). Do not request reserved bits, recorder flags or GNSS in this fallback. Attempt each group at most once during discovery, then poll only proven groups; fast polling includes the core fields so core freshness is preserved. Do not progressively guess layouts. Unknown categorical values or newly used reserved state bits become existing `unknown`/null fields without invalidating independent finite observations. Existing strict audited decoders remain unchanged.

Do not probe commands 10, 31/32 or unstable alerts command 35 on unknown releases. Consequently active firmware faults, fatal state, IMU-timeout state and alert history are unavailable unless supplied by a separately verified profile; do not fabricate inactive/clear observations or history. A stop-condition code is not current timeout evidence. Unknown package/ride state cannot drive riding-time accumulation, derived status or safety authorization; usable numeric samples can still be recorded. Bounded app-local data operations remain available under their existing rules.

A supported schema adds configuration reads independently. Failure of schema discovery, an optional numeric group or alert availability cannot tear down established core telemetry. Existing freshness, timeout, route and reconnect rules still apply; reconnect re-establishes evidence instead of inheriting prior permission. This change requires no persisted compatibility badge or reinterpretation of old sessions.

### 3. Limited support grants no device mutations

All board mutations are denied for limited support, including configuration command 95, Tune apply, package enable/disable 12, flywheel 22 (including disable), both lighting commands (including the zero-mask command-20 query), clear-fatal 36, remote/motor controls, recorder operations and VESC shutdown. Read requests still travel as transport writes; the restriction concerns device-changing semantics and command families, not BLE packet direction.

The existing central mutation owner checks verified capability both at admission and immediately before encoding/scheduler submission, including queued work after capability or generation changes. Direct public requests, stale frontend state and a matching schema cannot bypass it. Existing audited mode, IMU-timeout, revision, readback and flywheel-disable escape rules remain intact; the escape does not authorize an unverified command on unknown firmware.

### 4. Shared schema admission, exact observed identity

After existing bounded decompression and full XML validation, compute SHA-256 of expanded XML with only inner contents of text-only `ConfigParams/Params/<parameter>/longName` and `description` leaves replaced by empty bytes. Escaped text and CDATA are text; actual child elements in these leaves are rejected. Preserve tags, attributes and every other byte, including whitespace, comments, ordering, constraints, defaults, units, enums, grouping and unmodeled properties such as `maxLen`. Use the existing XML reader's stack/byte positions, not regular expressions or a general canonicalizer. A wire CRC alone never grants admission.

Compatibility owns the finite shared schema profiles and audited release-to-profile selection. For a known release require its selected schema profile; for limited support compare against the existing known read profiles without inventing a release entry. A matching key permits the existing signed value codec and schema/default reads; current reads additionally require fresh known normal mode. It never grants mutation semantics. A mismatch makes configuration unavailable while independent telemetry continues. Keep legacy full-fingerprint resolution only for ambiguous INFO `1.0` identities 1.0.0/1.0.1; delete the non-legacy configuration catalogue and duplicate exact-release lookup.

Schema identity owns actual observed/resolved version and full fingerprint strings. Keep the full-fingerprint algorithm, value wire signature and exact tuple used by public results, drafts, revisions, saved Tunes and complete write/readback. Cosmetic variants have different full identities: no alias, automatic conversion or cross-version/full-fingerprint Tune apply. Matching profiles do not merge namespaces. On limited support users can inspect/read compatible configuration and preserve data; applying it to the board stays unavailable.

Use one validation/admission path for downloads and cached raw XML. Retain existing exact firmware/hardware/package/index/nonzero-CRC cache identity and integrity envelope. Recompute full fingerprint, profile key and signature on a hit; never cache authority to write. Zero/missing/changed CRC or incompatible/corrupt bytes use the bounded fetch path. Upstream `main_calc_hw_crc` includes custom-configuration XML (`vendor/bldc/main.c`), so changed XML invalidates that identity. No storage marker/archive migration or serialized profile key is needed.

### 5. Minimal authoritative contract and frontend behavior

Add live `ActiveConnection.refloatSupport`, null before a decision and for BMS, otherwise `{ mode: 'verified' | 'limited', alertHistorySupported: boolean }`. Publish the decision atomically with readiness and feature availability in complete monotonic snapshots. Reconnect clears it. This is a protocol support level, separate from VESC access metadata, and is not saved into sessions or archives.

Reuse existing control availability/writability and configuration status/current/default-loaded fields. Add `ridingTuneApplySupported` to the small configuration summary, projected from backend policy; it is false for limited support and does not replace dynamic mutation guards. Add closed configuration write-block reason `unverifiedFirmware` when configuration is readable but mutation semantics are unverified. Earlier discovery/loading/unavailable reasons retain their priority. Remove frontend `refloat-tune-policy.ts` and its release list; frontend requests and presentation consume these authoritative facts.

A ready limited connection shows a persistent nonmodal explanation that live readings/recording are available and board changes are unavailable. Configuration and alerts present their actual availability. Do not require a firmware update or repeat a confirmation at each connection. Reserve the existing failed-connection recovery dialog for unusable protocols; its offline release list describes verified releases, not every connectable version. Keep native pointer/keyboard semantics and avoid command IDs in product copy.

Update contract, client/mock, shared projection, both host integrations and canonical cases together. Existing coarse C/JNI JSON boundaries should need no new native symbols. Bulk schema/values remain request-scoped; no generic permission matrix or host-owned compatibility decision is introduced.

### 6. Keep maintenance proportional to changed behavior

Retain exact audited beta2/beta3 mappings when 1.3.1 stable arrives. A new unknown beta4, stable 1.3.1 or stable 1.4.0 first tries the same public limited path without registration. A later audit can select shared verified profiles with a small mapping and focused evidence; only changed bytes/semantics warrant new profile code or fixture payloads. Stable branding alone neither upgrades an unknown release to verified nor removes known beta capability.

Extend the existing source-fixture generator/manifest with independently computed schema keys. Preserve verified byte reuse, source provenance and frozen supported-data fixtures. Represent hypothetical future identities and cosmetic variants as small bounded fixture transformations; do not copy full datasets or import production admission into the oracle. No generic overlay framework, release range or parallel registry is needed.

## Verification and rollout

Cover direct/CAN positive unknown identities, malformed/unsupported INFO and core responses, optional failure isolation, unknown status, no premature session activation, reconnect/generation cancellation and every mutation entry/dispatch path. Prove zero prohibited frames even with matching configuration, stale UI or queued requests. Keep exact beta2/beta3 controls and data access after a simulated stable registration. Cover cosmetic schema positives and protected-property negatives (including same CRC), cache lifecycle, current/default mode restrictions and unchanged Tune identity.

Run focused affected Rust and frontend checks, direct/mock contract conformance, relevant browser interactions and both native boundary consumers. Preserve old supported sessions/Tunes/archives. Synthetic future-version fixtures prove the fallback rules, not actual unreleased-firmware interoperability. Any physical checkpoint is selected for changed interoperability/host behavior that software cannot establish, using local evidence and safe restoration; do not make nonexistent future firmware a completion prerequisite.

## Risks and rollout

- A future release can retain a packet shape while changing meaning. Limit the fallback to documented reads, label its support level and require an audit or explicit future upstream contract for mutations.
- A protected XML edit can disable configuration even when telemetry remains usable. Keep the narrow comparison visible through feature availability; a new shared profile needs evidence rather than a broader normalization framework.
- Removing the frontend version list changes a public projection. Land contract/client/mock/shared/host changes together and verify complete snapshots before enabling the UI path.

There is no storage migration. Existing audited mappings are preserved throughout rollout; fallback permission is established afresh on every connection. Rolling back the application restores its prior admission behavior without rewriting supported data.

## Planning status

This revision is ready for review and remains open. All implementation tasks are unchecked. It supersedes the earlier no-contract-change, frontend-deferred and beta-removal decisions. No implementation, canonical spec sync, archival, commit or online action is authorized by this planning request.
