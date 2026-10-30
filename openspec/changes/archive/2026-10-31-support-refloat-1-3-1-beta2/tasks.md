# Tasks

## 1. Audited release and configuration evidence

- [x] 1.1 Pin `vendor/refloat` to immutable `v1.3.1-beta2` commit `ebe3eb5886b9f7c6247d9a06abeafa8415a94542`; record source comparisons with 1.3.0 and beta1 for INFO, commands 33/35, controls, stop code, and schema. Verify the source diff and tag commit match the recorded evidence.
- [x] 1.2 Add one exact beta2 compatibility entry and generic bounded suffix handling while preserving stable numeric-suffix admission; verify focused compatibility and direct/CAN discovery tests accept beta2 and reject beta1, beta3, stable 1.3.1, and malformed identities.
- [x] 1.3 Extend the existing schema fixture generator and exact catalogue for beta2, preserving old fixtures and same-version Tune isolation. Verify reproducible compressed schema, fingerprint, signature, 172 fields, current/default values, representative safe write/readback, and configuration codec/service tests.

## 2. Typed timeout telemetry and safety

- [x] 2.1 Extend command-35 decoding with explicit firmware-fault and IMU-timeout alert kinds, beta mask validation, and code-zero activation/clear; verify source-derived mixed, truncated, invalid, and older-profile vectors in focused decoder tests.
- [x] 2.2 Extend the existing telemetry service with independent timeout state, bounded history/cursor deduplication, stop code 7, current-generation freshness, and reset behavior. Verify active/clear, transient recovery, expiry, failed read, clock uncertainty, reconnect, and stale-response cases in focused tests.
- [x] 2.3 Gate package changes, flywheel enable, edited configuration writes, and Tune apply on fresh confirmed inactive timeout at shared submission; retain eligible flywheel disable and existing lighting/fatal rules. Verify rejected requests emit no traffic and refreshed inactive state restores only normal eligibility in mutation and coordinator tests.
- [x] 2.4 Update Refloat protocol and safety documentation for the audited beta profile and command-35 semantics; verify the documented commands, bounds, and exclusions against the pinned source.

## 3. Durable events and shared contract

- [x] 3.1 Add timeout activation/clear records to existing transactional session events and bounded live history, preserving source ticks and nullable occurrence time. Verify shared recovered-history limits, overlap deduplication, failed commits, paging, reopen, and guest-only ephemeral behavior in focused storage/service tests.
- [x] 3.2 Extend readers, integrity validation, archive import/export, and session-context version validation for beta2 provenance and timeout events; verify frozen baseline sessions/archives and new timeout-containing round trips without changing storage or archive format markers.
- [x] 3.3 Update the handwritten contract, shared projection, loopback history, client validators, and mock with typed timeout state, history, stop reason, event details, and actionable write-block reasons. Verify snapshot/event conformance and affected C/JNI consumer compilation without a parallel host implementation.

## 4. Existing UI and conformance fixture

- [x] 4.1 Present active timeout, concurrent firmware fault, retained stop reason, and activation/clearing records in the existing warning and renamed “Faults and errors” history experience. Verify relevant frontend unit and browser interactions, healthy-menu hiding, reload, guest history, and accessible labels.
- [x] 4.2 Extend conformance-owned beta2 direct/CAN fixture scenarios, using the connected ESP32 transport fixture where useful without moving scenario logic into firmware. Verify exact admission, live/saved telemetry, configuration, same-version Tune apply, malformed alerts, mutation gates, reconnect, and older-release regression in simulator and canonical direct/mock runs.
- [x] 4.3 Update reusable fixture and support documentation for beta2 plus the real-board checkpoint procedure; verify examples and references against the pinned source and current test commands.

## 5. Affected-consumer and physical integration

- [x] 5.1 Run affected Rust crate/consumer tests, warnings-denied Clippy and formatting; frontend checks/build; conformance TypeScript and available canonical direct/mock runs; relevant Apple and Android binding checks; and macOS-sandbox-external Playwright after the frontend build. Record actual results, resolve affected failures, and record unavailable host and unrelated test coverage before broadening coverage.
- [x] 5.2 On an exact beta2 board prepared by the user, run a scoped physical checkpoint on the affected available route/host: identity, fast/slow telemetry, saved logging, schema/current/default, safe configuration write/readback/restore, eligible same-version Tune apply, and reconnect. Keep complete backup/restore and private raw evidence under ignored `.local/physical-testing/`; publish a brief build/source-linked outcome and explicit coverage limits without inducing IMU failure.
- [x] 5.3 Run `git diff --check` and strict OpenSpec validation with `OPENSPEC_TELEMETRY=0`; review all completed tasks, supported-data evidence, and material omitted coverage. Use `just verify` only if remaining impact or a failure warrants it.

## Workflow follow-up

- After implementation and verification, sync and archive this change according to repository policy. Do not create commits or change a Pull Request without the user's explicit authorization.
- When stable Refloat 1.3.1 is released, audit it in a separate change, replace beta2 runtime admission and active beta fixtures, and preserve all supported beta-created stored data.
