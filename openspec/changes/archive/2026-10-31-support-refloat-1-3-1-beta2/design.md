# Design

## Context

See [proposal.md](proposal.md). `vendor/refloat` currently points to 1.3.0; the exact beta2 tag is `ebe3eb5886b9f7c6247d9a06abeafa8415a94542`. The beta1-to-beta2 source diff changes motor-control behavior and descriptive configuration text, but does not add another telemetry command, alert ID, or stop condition. Relative to 1.3.0, 1.3.1 introduces IMU timeout alert ID 2, stop condition 7, and configuration metadata changes. Command 35 is already used for bounded firmware-fault history despite stale exclusion text in some current specs.

The selected Rust runtime owns compatibility, telemetry, mutation policy, persistence, and public projection. Apple and Android use the existing bindings. The frontend contract is handwritten. Supported storage starts at baseline 100 and must not reset for a firmware release.

## Goals / Non-Goals

**Goals:** Add one reviewed beta2 release mapping and one reusable IMU-timeout alert path. Keep the existing telemetry scheduler, configuration codec, event store, history UI, and host bindings as the primary implementation. Keep the source release identity separate from reusable wire and control behavior.

**Non-Goals:** Automatically accepting future release labels; converting or applying Tunes across exact versions; duplicating the beta1 branch's sampled-versus-recovered timeout reconciliation; adding another polling lane, schema framework, or native host runtime. Refloat's runtime tuning, data recording, remote, and motor-control UI remain outside Float View's allowlist.

## Decisions

### One release decision, reused behavior

Decode the complete bounded INFO identity, including its suffix, once in the shared compatibility owner. Preserve the existing exception for stable builds that report the upstream numeric patch artifact as a suffix. Add exactly `1.3.1-beta2` to the reviewed catalogue and map it to the existing selectable command-33 and stable-control profiles. Select IMU-timeout capability from that catalogue entry rather than testing the beta string in downstream services. Neighboring betas, malformed suffixes, and stable 1.3.1 remain unsupported until audited. Preserve the complete accepted identity in display and session context. If an internal canonical compatibility record needs a new extent, version it once and update all same-build consumers; it is not an owned-storage marker.

An automatic `1.3.x` range would make unreviewed control and persistent-write semantics eligible. A beta-only decoder or parallel profile would duplicate 1.3 behavior. The catalogue entry is the deliberate release gate; the implementation remains family-based.

When stable 1.3.1 is published, audit its source and physical behavior, replace beta admission and active beta fixtures with stable identity and fixtures, and remove beta-only branches or tests that no longer serve supported behavior. Do not delete or reinterpret supported sessions, timeout events, provenance, or Tunes written while beta2 was supported. Exact-version Tune policy means a beta2 Tune is retained but does not apply to stable 1.3.1 automatically.

### Reuse 1.3 telemetry and decode typed alerts

Keep existing fast/slow command-33 masks, route selection, cadence, and command-35 history polling. The command-35 decoder accepts firmware-fault ID 1 and, only for the audited timeout-capable profile, IMU-timeout ID 2. The beta active mask permits bits 0 and 1; the extension mask remains zero. Firmware-fault activation still requires a nonzero fault code; timeout activation and clearing both use code zero, so `active` must be read independently. Reject inconsistent masks, IDs, record shapes, extents, and stale generations atomically. Include alert ID in bounded overlapping-read deduplication.

The active mask is authoritative for `imuTimeoutActive`; command-33 stop code 7 is a retained stop reason and cannot establish current active state. Initial, failed, or expired inactive command-35 evidence projects unknown. Keep active warning until a valid clear. Use an independent monotonic freshness clock for inactive alert evidence; fast samples cannot refresh it. Reset alert state and history cursor when route or generation changes. Do not infer a firmware fault or fatal latch from timeout ID 2.

### One existing history and persistence path

Persist recovered timeout activation and clearing as a distinct typed `session_event` kind in the existing transactional event store. Carry source ticks, observed time, and nullable approximate ride occurrence time; never present clock-derived time as exact. Share the existing 10,000-record recovered-alert budget and 20-record live history bound. Guest history remains ephemeral. Expose the new event in the existing authenticated event pages and archive/export readers, with strict validation, while preserving old firmware-fault event shapes and baseline fixtures. A storage marker or archive-format increment is not planned because the current extensible event format can carry a new kind; implementation must prove old and new data round trips before relying on that assumption.

Persist the sampled `imuTimeout` stop transition separately through the existing sample-event pipeline. Use command-35 records as the alert-history source rather than building another sampled/recovered reconciliation algorithm. Source buffering and polling can miss an episode; the UI must describe history as recent and approximate. The existing Fault history overlay and list gain inclusive “Faults and errors” labels, an active timeout warning, and distinct activation/clearing rows. Keep a firmware-only field available for existing consumers; project timeout fields separately in the handwritten contract and mock.

### Safety at submission

For timeout-capable releases, package state changes, flywheel enable, edited configuration writes, and Tune application require fresh confirmed inactive timeout state at scheduler submission. Active, unknown, failed, or expired state blocks them before traffic. Flywheel disable retains its existing escape path; lighting retains its existing access and mode gates. Timeout alone does not enable fatal-clear command 36. Add actionable backend write-block reasons for active and unconfirmed timeout so frontend guidance mirrors the shared mutation decision. Existing revision, ride-state, mode, access, and complete readback gates remain in force.

### Exact beta2 schema for this change

Pin beta2 source, extend the existing fixture generator, and use its exact compressed schema, values, fingerprint, and signature as independent test evidence. The source XML SHA-256 is `6bc6bfa433f8205519bc931723e39252aa71c9f538a0ed7c200d688486c63e73`; its 172 serialized fields and `0x191a6c1b` wire signature match the audited comparison. Reproduce the generated bytes and catalogue identity during implementation. Keep the current full-fingerprint gate and exact-version Tune namespace, command-95/readback verification, schema cache validation, and complete current/default reads. The separate unfinished `relax-refloat-schema-admission` change owns any later change to the fingerprint policy.

## Risks / Trade-offs

- [Upstream beta source differs from test fixture] → Pin the full tag commit, derive fixtures from it, and compare generated bytes and command behavior with source before enabling admission.
- [A timeout occurs between slow history reads] → Use the existing bounded source history and cursor; keep times approximate and report the 20-record source limit rather than fabricating completeness.
- [Inactive evidence becomes stale while fast telemetry continues] → Expire it independently, invalidate on failed reads, and recheck at mutation submission.
- [New event kind reaches supported storage and archives] → Additive readers and validators must accept old and new records; prove baseline reopen, import/export, and failed-commit behavior before concluding no marker change is needed.
- [A beta-specific policy survives stable release] → Keep the release identity in one catalogue and document the replacement audit; retain supported historical data without retaining beta runtime admission.
- [Real IMU failure is unsafe to induce] → Use source-derived simulation and ESP32 transport fixture for activation/clear and malformed cases; the real beta2 board validates ordinary interoperability without a hazardous induced failure.

## Migration Plan

No user Tune conversion or automatic configuration migration occurs. Implement source evidence and exact identity, then typed alert decoding/state, durable history and contract/UI, followed by conformance and targeted hardware checks. The connected ESP32 fixture supports repeatable protocol development; Refloat scenario logic remains in `conformance/`, not ESP32 firmware. A later exact-beta2 board validates discovery, live and saved telemetry, configuration/current/default, a backed-up safe write and restore, same-version Tune apply, and reconnect on the affected available host/route. Keep raw hardware details under ignored `.local/physical-testing/` and publish only a brief source/build-linked outcome.

Rollback may stop admitting beta2, but must leave supported stored sessions, timeout events, archives, and Tunes readable. A later stable-release change replaces active beta support after its own audit and tests; it does not erase beta-created user data.
