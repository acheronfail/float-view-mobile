# Tasks

## 1. Typed durable context

- [ ] 1.1 Add the versioned closed context payload and validation with separate controller/peer facts and direct/CAN routing; verify direct, Express/CAN, controller-gateway, test firmware, null names, private-field rejection and byte/version/route bounds with focused Rust tests.
- [ ] 1.2 Add independent context append/count/read support to the existing session writer, including activation operation-ID idempotency, 1,000-event admission and atomic commit markers; verify duplicate retry at capacity, failed/uncertain writes, zero-sample sessions and equal-time ordering with writer/reader tests.
- [ ] 1.3 Audit supported schema validation without changing frozen fixtures; verify baseline/current session databases open unchanged with zero context, and document the additive event layout and bounds in backends/sql/README.md.

## 2. Activation and recovery

- [ ] 2.1 Capture fresh ActivationFacts through the attached runtime before saved readiness/telemetry, restoring the timeline first on resume; verify initial capture, unchanged reconnect, changed firmware/route and stale/duplicate completions with service/runtime tests.
- [ ] 2.2 Preserve context and activation identity through committed-write recovery while recording a new event for a new process activation; verify interrupted commit, resumed timeline, WebView replacement without activation, guest exclusion and failed identification.
- [ ] 2.3 Integrate context admission/storage failures with existing actionable recording failures; verify no ready state or telemetry append for an unrecorded activation and no reset, history eviction or automatic session replacement at capacity.

## 3. Shared public reads

- [ ] 3.1 Extend frontend contract/client/validators/mock and shared server dispatch/projection together with connectionContextCount and snapshot-scoped connectionContexts pages; verify type checks, focused client/mock/server tests, at most 64 rows and 262,144 encoded bytes, cancellation and forward progress at maximum payload size.
- [ ] 3.2 Add canonical conformance cases for authentication, empty/old recordings, ordering, cursor ownership, active snapshot refresh/conflicts and history after reconnect; verify affected mock/direct scenarios and both existing Apple/Android contract integrations using the shared implementation.
- [ ] 3.3 Document the new session-data kind, payload and pagination semantics alongside existing contract/read documentation; verify examples against contract types and the conformance requests.

## 4. Maintenance and portable archives

- [ ] 4.1 Preserve all context events through telemetry/location pruning and commit-boundary recovery without protecting unrelated samples; verify prune/reopen cases with context before the first sample, between deleted samples and after the last sample.
- [ ] 4.2 Validate and preserve context in native archive export/import, including operation IDs, exact event identities/times and ordering; verify round trips after pruning, interleaved event families, malformed/oversized/over-cap payload rejection and no partial publication.
- [ ] 4.3 Verify supported baseline archive imports and existing format-4 exports without context, public firmware context without private identifiers, unchanged telemetry-only CSV and GPS-independent context retention; update storage/export documentation with the tested compatibility result.
- [ ] 4.4 Verify context-only sessions still follow ordinary no-riding/minimum-distance discard, marker/fault exemptions remain effective, and explicit session/board deletion removes context with owned data.

## 5. Session details

- [ ] 5.1 Add the compact recorded firmware/path section and chronological paged initial/reconnect entries to active/completed session details; verify direct and gateway versions, test firmware, missing names, unchanged reconnects, Not recorded and later-resume-only context with component/mock tests.
- [ ] 5.2 Keep bounded page/request state and existing chart/map inspection during metadata refresh; verify stale responses, session switches, snapshot conflicts, cancellation and primary/non-primary/keyboard activation in focused tests and affected browser coverage.
- [ ] 5.3 Update session-review documentation for connection details and archive portability; verify its behavior descriptions against the delivered UI and missing-context states.

## 6. Affected-consumer verification

- [ ] 6.1 Run affected Rust crate/consumer tests with locked offline dependencies, formatting and warnings-denied Clippy; run frontend checks, focused tests and formatting/lint checks, then build before affected browser tests outside the macOS sandbox; record actual results.
- [ ] 6.2 Run affected canonical mock/direct conformance and existing Apple/Android shared-boundary checks; verify cross-platform archive/context equivalence and report any unavailable relevant integration coverage, adding scoped physical checks only if implementation changes BLE/OS behavior software cannot establish.
- [ ] 6.3 Run strict OpenSpec validation and git diff --check, review baseline/archive compatibility evidence and keep task updates in coherent implementation commits; verify all tracked work is complete before finalization.

## Workflow follow-up

- Commit completed planning artifacts before implementation, following the repository's three-phase OpenSpec workflow.
- After implementation and relevant verification, sync delta specs and archive this change together in the final commit; preserve frozen fixtures and independent storage/archive compatibility policy.
