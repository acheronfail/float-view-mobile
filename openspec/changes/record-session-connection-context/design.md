# Design

## Context

See proposal.md for motivation. `ActivationFacts` already combines the selected routed controller, transport firmware identity and exact Refloat wire version. `prepare_activation` creates or resumes saved sessions and attaches the existing application runtime writer. Context currently reaches bounded diagnostics, which have a different lifetime and purpose.

Sessions already have independent `session_event` rows, atomic event commit boundaries, snapshot-scoped readers, and portable archive format 4. Existing telemetry-event reads filter a closed set of transitions. Device snapshots in the catalog/archive describe board ownership and should not become a mutable connection-history store.

## Goals / Non-Goals

Capture authoritative protocol facts with the recording owner and expose them through the same shared service on all hosts. Describe the actual path to the selected motor, not the complete discovered bus. No native product policy, new protocol queries, dependencies, generated API layer, or duplication of context into every sample. CSV remains telemetry-only; full provenance travels with native archives.

## Decisions

### Store independent typed session events

Use kind `connectionContext` in `session_event`, with `source_sample_sequence` null, observed epoch time and session-relative monotonic elapsed time. Persist payload version 1, exact Refloat version and separate controller/transport firmware facts. Firmware facts retain VESC major/minor (rendered using existing version formatting), test version and nullable public hardware/firmware names. The controller route is `{kind: direct}` or `{kind: can, canId}`. Direct connections still record both roles explicitly, even when their facts match.

Express is the peer's firmware identity, not a third route kind. Keeping both roles also describes forwarding through another motor controller and avoids confusing gateway firmware with motor firmware. Use existing validated identity bounds, tightened to public names of at most 128 UTF-8 bytes; validate the complete encoded event at 4,096 bytes. Do not copy controller UUIDs, addresses, advertised/user names or bus inventory. A closed typed payload prevents arbitrary JSON becoming an extension channel.

Alternatives: a single catalog field loses reconnect history; sample columns repeat unchanged data; diagnostic rows expire and are intentionally payload-free. Reusing independent events avoids each of those problems and a new schema object.

### Commit context at the activation barrier

After choosing the saved session and restoring its timeline, build context from the current `ActivationFacts` and append through the attached runtime/session writer before activation permits telemetry persistence or publishes ready state. Update event sequence/count, committed elapsed boundary and history revision atomically. Context may exist before any sample and must not depend on a sample foreign key. Preserve the current catalog reconciliation and recovery ownership.

Assign a bounded operation ID to the activation, retain it across retries of that activation, and persist it with the event so an uncertain append can be resolved without duplication. Reject stale activation/session generations before appending. A genuinely new activation, including after process recovery, gets a fresh ID and records a new event even if its facts are identical. A recovered committed activation event remains historical evidence; it does not claim uninterrupted connectivity. Mere WebView/listener replacement does not create an activation.

Use restored session elapsed time on resume, including existing offline-gap semantics; equal elapsed times are ordered by global event sequence. Failed identification and guests never append. Preserve existing controller continuity decisions: context capture neither forces a same-session resume nor merges sessions from different controllers.

Limit admission to 1,000 context events per session, independently of other event families. Check duplicate operation IDs before capacity checks. Capacity or append failures use the existing recording/storage failure flow and prevent saved readiness for that activation. Never silently continue recording with missing context, truncate older history, or create a replacement session to bypass the cap.

### Add a dedicated bounded historical reader

Add mandatory session-data kind `connectionContexts` rather than widening telemetry transition pages or embedding arrays in `session.get`. Metadata adds `connectionContextCount`, computed within its committed snapshot using existing indexed event boundaries. Readers use bounded independent connections and the existing session ID/snapshot/cursor conventions, returning elapsed time, observed time, event sequence and typed context. The operation ID is storage bookkeeping and need not be exposed publicly.

Pages contain at most 64 events and at most 262,144 encoded UTF-8 bytes including envelopes; fill to whichever limit is reached first and require forward progress. Reuse current request/body and cancellation limits. Order by elapsed time/event sequence, exclude rows beyond the snapshot boundary, and validate cursor ownership. Do not add historical arrays to complete AppState snapshots. Update contract, validators, client/mock, shared dispatch/projection and both host consumers together.

### Show firmware and reconnect history in session details

Add a compact connection-details section to the existing active/completed session viewer. The first entry is the initial recorded connection; later entries are reconnects, each with session elapsed time and the Refloat/controller versions and path. If capture began on a later resume, label it as the first recorded connection without claiming it describes the earlier ride. Show test-version information when nonzero and peer firmware separately for CAN connections. Render public text as text, never HTML. Missing context reads Not recorded, and missing names use neutral labels rather than inferred Express classification.

Use page-sized history navigation, keeping at most the displayed page and bounded request state. Reuse active-session metadata refresh, request cancellation and stale-response guards; refresh context without remounting charts/maps or resetting inspection. Keep any section/navigation controls native buttons with primary Pointer Event and guarded accessible click handling per repository guidance. Do not require expanding/loading context history to use the rest of session review.

### Preserve events through maintenance and archive validation

Pruning retains every context independently of protected sample selection; contexts have no sample dependency. Recovery preserves committed context, operation IDs and event boundaries. Do not add context to the existing marker/fault automatic-retention exemptions, so ordinary unsuccessful short rides remain discardable. Explicit deletion keeps its existing ownership semantics.

Native archive copies retain context in the session database, not an additional manifest history or board snapshot field. Validate payload version, closed fields, operation-ID uniqueness, types, timestamps/timeline ordering, versions, route IDs (0–253), UTF-8 byte limits and the context cap before importing/publishing; malformed context fails the import without partial catalog publication. Preserve normal interleaving with other event kinds and exact event identities/times across export/import. Diagnostic redaction and GPS-export settings do not strip connection context.

## Risks / Trade-offs

- Reconnect storms consume history capacity → explicit cap and actionable activation failure; no ring buffer that silently loses provenance.
- A new failure point exists before ready → make context commit atomic/idempotent and exercise failed/uncertain-write recovery and retry behavior.
- Context events can advance elapsed time without samples → test metadata bounds, recovery and zero-sample review without attributing riding time or distance.
- Existing archive validators and readers may assume known event kinds → audit validation and pruning explicitly, and test supported frozen fixtures rather than assuming generic JSON storage guarantees compatibility.
- Bounded page filling could exceed the byte limit at maximum payload → size including envelope/cursor and stop before the limit, with tested forward progress.

## Migration Plan

Use the existing supported session schema and archive format 4 if tests confirm this additive event vocabulary requires no layout change. Existing recordings remain valid with count zero; do not backfill or modify frozen baseline fixtures. A resumed old recording gains context only at its new activation. Preserve baseline 100 and current supported storage upgrades.

Implement shared capture/reader/archive validation and the contract/client/mock/UI in coherent verified slices, with the completed planning artifacts committed first. Rollback must preserve files containing context and must not run compatibility reset logic. Do not promise that older app binaries display the new event; verify whether they can safely retain it. If implementation discovers an incompatible layout is necessary, revise this design before coding it, increment the owned-storage marker with a crash-safe tested supported-data upgrade, and handle archive numbering independently while preserving baseline imports.

## Verification

Use focused Rust runtime/service/writer/reader/pruner/archive/recovery tests and affected consumer checks; canonical mock/direct conformance covers authentication, snapshot pages, reconnect/route changes, failures and archive round trips. Frontend tests cover recorded facts, missingness, bounded history, stale requests and input activation; build before affected browser tests, which run outside the macOS sandbox. Verify both existing host contract integrations without introducing native policy. No new BLE/OS behavior is intended; require a scoped physical checkpoint only if implementation changes a boundary software cannot establish. Planning-only validation is strict OpenSpec validation and diff/link review.
