## 1. Pin source and derive beta evidence

- [x] 1.1 Resolve `v1.3.1-beta1` to its full immutable commit, pin `vendor/refloat`, and record the audited INFO suffix, command-33 fields, command-35 masks/IDs, stop code 7 and existing control payloads; verify against the source diff while retaining old tags and fixtures.
- [x] 1.2 Extend the existing configuration-fixture generator for exact `1.3.1-beta1`, derive fingerprint/signature and values, and add its catalogue entry; verify reproducible fixture digests, serialized order/count, size limits, metadata changes and codec round trips without changing frozen baseline fixtures.
- [x] 1.3 Add beta identity, direct/CAN telemetry/control and mixed firmware-fault/timeout source vectors; verify command-35 activation and clear both permit timeout code 0 and match the source's mask and record encoding.

## 2. Admit only the audited beta

- [x] 2.1 Extend the existing suffix-aware closed compatibility representation, canonical records and affected consumers to admit exact `1.3.1-beta1`; verify acceptance and round trips plus rejection of stable 1.3.1, beta, beta2, malformed suffixes and ranges while replaying all older catalogue entries.
- [x] 2.2 Carry the full identity through discovery, capability decoding, supported-version guidance, configuration cache and Tune matching; verify direct/CAN readiness, replacement-generation rejection, exact schema matching and isolation from 1.2.x/1.3.0 Tunes without a conversion path.
- [x] 2.3 Reuse the existing 1.3 selectable telemetry and stable control profiles; verify canonical fast/slow/GNSS requests, existing history cadence, no duplicate fault-probe lane, unavailable-value nulls, package/flywheel/lighting/fatal confirmation and complete configuration write/readback.

## 3. Decode and coordinate IMU timeout safely

- [x] 3.1 Replace firmware-only alert-record assumptions with explicit firmware-fault and timeout kinds, beta mask validation and stop code 7 mapping; verify mixed active masks, code-0 activation/clear, names, unsupported IDs, every truncation boundary, trailing bytes and malformed input produce atomic outcomes.
- [x] 3.2 Extend generation-scoped telemetry state, history cursor and bounded deduplication to include alert kind and independent active timeout state; verify same-tick mixed records, overlapping responses, transient recovery, occurrence-time uncertainty, source-clock boundaries, timeout scheduling, reconnect resets and stale replies.
- [x] 3.3 Add timeout-aware gates at the shared mutation boundary; verify affected writes emit no traffic during an active timeout, guarded flywheel disable remains available, timeout alone never enables fatal clear, and clearance restores only ordinary eligibility.

## 4. Preserve durable history and the shared contract

- [x] 4.1 Define `imuTimeoutActive`, the `imuTimeout` stop literal and `imuTimeoutHistory` details in the hand-written contract; update shared projection, client/mock validators and affected C/JNI consumers together, verifying identical authenticated snapshots and event JSON across both host integrations.
- [x] 4.2 Add timeout records to existing transactional event storage, readers, integrity validation and export/import; verify the shared 10,000-record recovered-history budget, ordering, deduplication, failed commits, sampled transitions, reopen and paging without polluting firmware-only history.
- [x] 4.3 Prove compatibility with supported baseline storage, existing Tunes and baseline archives using frozen fixtures plus new round-trip cases; document whether the additive format needs no marker change, or revise the plan and implement a crash-safe upgrade before any actual incompatible transition.

## 5. Present and simulate the behavior

- [x] 5.1 Add live active-timeout and retained-stop labels plus historical activation/clear presentation; verify frontend unit/mock cases for concurrent firmware faults, cleared timeouts, unavailable occurrence time and frontend recreation, and relevant browser coverage for warning/history behavior.
- [x] 5.2 Extend the canonical simulator and scenarios for exact beta schema, commands 33/35 and existing controls; verify direct/CAN, saved/guest, mutation rejection, malformed responses, transient timeout history, reconnect, durable queries and export through the shared service without adding ESP32 scenario logic.
- [x] 5.3 Update reusable support/protocol/checkpoint documentation and remove superseded firmware-only assumptions; verify documented commands, exact supported-version guidance and explicit exclusion of Tune conversion, runtime tuning and later betas.

## 6. Verify affected consumers and interoperability

- [x] 6.1 Run focused affected Rust crate and consumer tests with `cargo test --locked --offline -p <crate>` from `backends/core`, relevant formatting and warnings-denied Clippy, and fixture/conformance TypeScript checks; record actual commands/results and resolve failures before widening scope.
- [x] 6.2 Run affected frontend tests, `npm run check`, formatting/lint checks and build sequentially; run relevant Playwright files outside the macOS sandbox after the build and record results.
- [x] 6.3 Run affected mock/direct canonical conformance and Apple/Android integration checks for the contract and existing bindings; demonstrate equal beta behavior and regressions for older profiles, and expand native build or runtime coverage only where affected.
- [x] 6.4 Perform a targeted interoperability checkpoint with actual `1.3.1-beta1` firmware for discovery, fast/slow telemetry, configuration/readback, eligible controls and reconnect on relevant available routes/hosts; record source/build-linked outcomes and missing required coverage, explicitly separating simulated timeout evidence from real firmware IMU failure. Keep detailed evidence under ignored `.local/physical-testing/` and sanitized outcomes public.
- [x] 6.5 Run `git diff --check` and strict OpenSpec validation with `OPENSPEC_TELEMETRY=0`; review completed task evidence and material coverage limits. Use `just verify` only if remaining impact or failures justify broader regression.

## Workflow follow-up

- Follow the planning, implementation and final sync/archive Git phases only when commits are explicitly authorized; implementation is authorized, but commits require explicit authorization.
- After implementation and relevant verification, sync delta specs and archive the completed change together according to repository policy.

Implementation checks and material coverage limits are recorded in [verification.md](verification.md). Task 6.4 passed on actual Thor300 beta1 hardware through CAN over macOS BLE; unexercised routes/hosts and real IMU failure coverage are explicit in that record.

## 7. Review follow-up

- [x] 7.1 Require a confirmed inactive timeout state before guarded beta mutations; retain older firmware eligibility and flywheel escape.
- [x] 7.2 Display sampled timeout activation/clear history, suppress initial healthy clears, retain timeout-only rides, and deduplicate recovered observations.
- [x] 7.3 Expose bounded IMU history separately in live snapshots; show Fault history only for active faults, a latch or actual history, including cleared guest timeouts.
- [x] 7.4 Verify affected shared, frontend and conformance behavior and document stable-release replacement policy.

## 8. Review regression fixes

- [x] 8.1 Expire inactive command-35 authority independently of fast telemetry, invalidate failed reads, retain active warnings, and recheck freshness at guarded mutation submission; cover failure, recovery, cadence and older profiles.
- [x] 8.2 Keep retained-stop occurrence times unknown in recorded history and cover observation-only fallback presentation.
- [x] 8.3 Reconcile delayed recovered timeout transitions against ordered observation windows, preserving distinct episodes and paging; add regressions and run affected checks.

## 9. Production-code review follow-up

- [x] 9.1 Separate retained-stop evidence from authoritative sampled timeout state; simplify history parsing/reconciliation with named types and cover initial healthy readings, paging and later actual transitions.
- [x] 9.2 Use the compatibility profile's IMU-timeout capability for shared decoding and mutation gates; run focused frontend and shared-consumer checks, formatting/lint, browser smoke coverage and strict OpenSpec validation.

## 10. Generic wire suffix identity

- [x] 10.1 Replace beta-specific version fields/constructors and duplicate INFO suffix storage with a validated fixed-width suffix, preserve exact catalogue admission, and move IMU-timeout capability onto entries. Update and validate the internal compatibility record and affected consumers.
- [x] 10.2 Cover empty, beta, other bounded suffixes, invalid bytes/padding, catalogue capabilities and canonical records; run affected Rust/consumer tests, formatting/Clippy, exact-beta direct conformance and strict OpenSpec validation.
