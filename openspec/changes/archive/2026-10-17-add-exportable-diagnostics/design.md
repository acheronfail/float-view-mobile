# Design

## Context

See [proposal.md](proposal.md) for motivation and scope. `backend-service/src/diagnostics.rs` already provides payload-free event recording, metrics, a 256-event/30-second ring, anomaly capture and an asynchronous writer. However, `Diagnostics::start` requires a session lease and `event` drops everything without that sink. Collection therefore misses initial discovery failures, guest connections and errors before saved activation. Retention follows the recording, including deletion of provisional rides. `DiagnosticWriter` writes bounded JSON into `session_diagnostic`; recording archives preserve these rows along with potentially private ride data.

`backend-service/src/files.rs` already owns correlated, bounded file work and presentation leases. The shared server owns dispatch through existing C/JNI families. Native hosts provide environment/OS facts and file presentation. Public snapshots intentionally exclude diagnostic windows. The current Support link opens GitHub issues, and `PRIVACY.md` already explains that those issues and attachments are public.

## Goals / Non-Goals

**Goals:** Make failure context available without recording; preserve useful causal ordering through reconnects and process restarts; make default reports suitable for deliberate public sharing; keep collection isolated from product storage and real-time work; reuse existing owners and remove the superseded session-bound collection sink.

**Non-Goals:** No new runtime, native product policy, analytics service, crash SDK, arbitrary log viewer, protocol replay mechanism, detailed packet mode or product-data migration. Full process crashes can lose the most recent uncommitted events. Native startup recovery remains authoritative if the shared service cannot be constructed; reporting from that recovery surface is outside this change.

## Decisions

### 1. One shared journal, initialized independently of recording

Refactor the existing diagnostics owner inside `backend-service`, without adding a workspace crate or second application runtime. Initialize its in-memory ring and independent best-effort writer when the service starts, before connection work. Keep the current metrics, request/operation correlation and anomaly signals; replace the session lease with a diagnostic-store lease. Collection stays active for saved and guest connections, after disconnect, and without a WebView.

Derive a fixed private diagnostic directory from the protected root supplied by the native host, alongside rather than inside product storage: Apple `Application Support/FloatView/Diagnostics`; Android `files/float-view/diagnostics`. Root setup, file protection and platform facts remain native capabilities; journal encoding, validation, retention and report policy remain Rust-owned. Diagnostic file work uses the existing shared file-worker/presentation machinery generalized for service-issued diagnostic files, without acquiring a session lease or consulting the catalog. The directory must have exclusive ownership and reject links, path escapes and unrecognized layouts. Diagnostic initialization failure degrades to memory-only collection and never prevents an otherwise valid product service from starting.

This isolates running-service storage failures from diagnostic staging. It does not promise a working frontend when product-service construction itself fails. Keeping the journal in each session was rejected because it loses exactly the pre-session evidence this feature needs; a second native logger was rejected because it duplicates policy and diverges across hosts.

### 2. Typed events and sanitized facts at the producer boundary

Use a closed serializable event model rather than passing arbitrary JSON or formatted errors to a generic log API. Preserve command kinds, lengths, response/queue times, closed failure enums and lifecycle facts. Add connection discovery/selection failures currently discarded or collapsed in shared policy, schema compatibility failures, catalog/session/export failure categories and frontend mapped failures. Do not log every advertisement or successful sample; use existing aggregate timing windows and counts.

Each event has a format version, run-local sequence, monotonic elapsed time, run ID, optional connection generation, controller reference and operation sequence. Run IDs and controller references are random/ordinal diagnostic references, never hashes of private identities. Export maps them to report-local `run-1` / `controller-1` aliases. `generation` identifies the transport connection, while `application_generation` identifies the controller protocol generation; requests/activations retain both where available and export rebases the namespaces independently. Cross-run ordering uses committed journal order, not a comparison of unrelated monotonic clocks. Wall-clock time is used privately for expiry, not exported; a clock rollback cannot extend retention indefinitely because byte/count limits still apply.

Environment includes validated build version/revision/channel, normalized platform/OS/device model, recognized controller hardware model, firmware/package version, selected wire family, direct/CAN route kind and normalized schema identity where available. Unknown strings become `unknown`, with structural numeric facts where useful. Arbitrary firmware/model/version text is not copied merely because it is bounded or display-safe. Include no advertised/custom names, MACs, UUIDs, logical board IDs, BMS identity, coordinates (including board GNSS), telemetry values, tunes, whole configuration, secrets, paths or raw payloads. Defaults exclude unrelated device inventory. Invalid configuration values are described by closed field/category and structural bounds, never by rejected content.

Frontend global error/rejection handlers and selected feature error boundaries map failures into closed categories/features plus optional numeric source positions. They do not serialize Error objects, messages, stacks, URLs or DOM content. Existing `diagnostics.report` timing ingestion continues. All diagnostic request failures use a guarded no-report path to prevent recursion. An in-memory UI queue of at most ten entries can survive a temporary backend disconnect within the current WebView; it does not persist in browser storage.

Raw text followed by regex redaction was rejected: unknown firmware strings and exceptions can contain any private value. Closed event construction is the privacy boundary; export repeats schema validation as defense in depth.

Central request failure capture runs after authenticated dispatch, before returning
any stable error, including validation/state/permission/not-found/conflict errors.
Operation names use a closed allowlist; unknown names become unknown. Session HTTP
read/import/export errors use fixed route categories. The service records no
parameters or formatted error messages. The client queues only closed categories
for timeouts, disconnections and HTTP/body failures, with ten pending entries and
ten admissions per minute; normal requests/reconnection retry them. Diagnostic
requests and deliberate HTTP read cancellations are excluded. UI exceptions and
transport failures retain separate bounded queues under existing service limits.
Coalesced backend failures remain in the incident within its event/byte budget
after its follow-up window closes, without extending the window.

### 3. Bounded journal and incident lifecycle

Use independently versioned structured diagnostic files, not another product SQLite schema. Append to size-bounded journal segments; publish incident metadata and prepared output atomically. A partial final record after interruption can be recognized as an incomplete tail without altering earlier valid bytes. Unknown versions or malformed committed records are preserved and reported as unavailable/partial; never infer permission to reset a directory. Enumerate only bounded known filenames under the owned root and refuse unexpected links/layouts. Age pruning touches only validated supported diagnostic files. Document the new directory outside the product baseline/reset inventory.

Enforce these initial limits centrally and test admission before allocation:

| Resource                           | Limit / policy                                                                                                                                                   |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Recent in-memory context           | 30 seconds, at most 256 events, each at most 1,024 bytes                                                                                                         |
| Background writer admission        | Eight data jobs (event batches of at most 32 records or one bounded incident snapshot), plus one reserved clear job; nonblocking send and dropped-record counter |
| Journal rotation                   | Two segments, at most 2 MiB each                                                                                                                                 |
| Retained incidents                 | Three, at most 1 MiB each; journal + incidents + metadata at most 8 MiB                                                                                          |
| Retention age                      | Seven days, checked on startup and maintenance                                                                                                                   |
| Incident window                    | Available 30 seconds before trigger and ten seconds afterward                                                                                                    |
| Incident concurrency               | One open capture; automatic repeats within 60 seconds coalesce; manual admission once per ten seconds                                                            |
| Timing metrics                     | Existing 24 metric keys and last 256 observations per metric                                                                                                     |
| UI error ingestion                 | Ten per minute per client; 30 per minute service-wide; ten pending UI events                                                                                     |
| Prepared reports                   | Two globally; client-owned; ten-minute TTL; preview at most 32 KiB, summary at most 8 KiB                                                                        |
| Output                             | At most 5 MiB per ZIP; two staged/preparing/presenting report slots totaling at most 10 MiB                                                                      |
| Whole diagnostic directory         | At most 20 MiB including journal, incidents, staging, metadata and temporary files                                                                               |
| Worker execution                   | Reuse the existing two-worker file limit; at most two diagnostic preparations within the existing worker limit; existing stricter transfer limits apply          |
| Diagnostic writer retry / shutdown | At most three attempts; at most 5 ms between retries; shutdown join at most two seconds                                                                          |

Rotate before appending, evict oldest eligible finalized incidents before admitting another, and reserve room for metadata before writing output. Existing native presentation leases pin output; capacity fails with `unavailable` instead of evicting pinned files. Journal bytes are sanitized but still private app files. No indefinite collection extension follows repeated errors: coalesce counts/reasons without extending the original ten-second post-window. A manual mark during an open window joins that capture and returns its ID; requests exceeding the manual rate limit fail before mutation.

Persist capture-start/pre-context promptly, then mark completion after the ten-second window. If background execution is suspended or the process stops, retain committed records and mark the unfinished window incomplete on reopen. Do not acquire a background execution entitlement or keep the radio active for diagnostics. Incident list refresh is request-based while the reporting UI is open; diagnostics alone do not advance `AppState` revisions.

Clearing increments an internal diagnostic generation, drains/invalidates old queued work, clears the ring and retained files and invalidates unpresented report handles before returning. Late workers cannot restore cleared data. Active native presentations retain their leased file until completion/expiry; external copies remain user-owned. Collection can resume in the new generation without saving a deletion event containing prior data.

### 4. Explicit report preparation separates preview from sharing

Add the methods in the backend-contract delta. `diagnostics.prepare` freezes either a selected incident's available boundary or current recent context. It never waits for post-window completion: an early preview explicitly reports that limitation. It builds the sanitized ZIP in a bounded file worker and returns only the report handle, preview and metadata. Preparing again creates a distinct report. The UI explicitly supplies its previous `replaceReportId` to release that preview before preparing another; a pinned native presentation keeps its slot until completion. A report is bound to the requesting transport client, and lost/expired handles require a new preview.

`diagnostics.export` presents those exact bytes using the existing correlated export lease; it cannot silently rebuild from newer context. Extend service-issued staging ownership to the diagnostic root without exposing paths to the frontend. Reuse native save/share completion handling and its existing idle/presentation deadlines. Clients cannot choose output filenames, roots or arbitrary GitHub destinations. Reject path inputs, unknown params, foreign handles and excessive work through existing stable errors; retain only bounded error codes/messages.

The ZIP has three fixed root members:

- `report.md`: concise sanitized build/environment, available hardware facts, incident reason, timeline summary, report ID and limitations.
- `events.jsonl`: validated chronological event records with report-local aliases and relative times.
- `manifest.json`: report schema version, report ID, included categories, exact capture boundary, incomplete/clipped flags, aggregate loss counts and privacy omissions.

Use the pinned Rust `zip` 6.0.0 writer with default features disabled and stored entries; update licensing notices for its resolved dependency graph. The new dependency replaces bespoke ZIP header, CRC and directory encoding. Compression and ZIP reading are unnecessary for these bounded text bundles; existing gzip helpers do not produce ZIP archives. No crash, network or analytics dependency is added.

### 5. One export button and manual reporting instructions

Settings exposes one Export diagnostics button. It automatically lists incidents,
prepares the newest retained incident (or recent context when empty), then opens
native save/share. There is no visible preview, incident picker, manual mark,
refresh, copy-summary or clear control. A repeated export replaces its prior
prepared handle; native presentation leases remain valid. Busy/error feedback
keeps retries straightforward. Existing backend capture/clear/preview operations
remain supported without changing the public contract.

Concise static copy explains that logs exclude routes/device identifiers, nothing
is uploaded automatically, and public GitHub issues require manually writing
reproduction steps and attaching the ZIP. Detailed capture limitations remain in
the ZIP. All controls use primary Pointer Events with cancellation and guarded
keyboard/assistive clicks.

Add `.github/ISSUE_TEMPLATE/bug_report.yml` with stable field IDs: `summary`, `steps`, `expected`, `actual`, `hardware`, `diagnostics`. Require useful reproduction/expected/actual descriptions while allowing unknown hardware and a report unavailable explanation. Explain manual ZIP attachment and public visibility. Show the fixed repository Issues URL with guidance to check existing issues, write reproduction steps and expected/actual results, and manually attach the ZIP if the problem is a reproducible bug. Do not prefill issue fields or launch a new-issue composer. An ordinary link to the Issues list is sufficient; opening it never submits anything. This intentional friction lets the user decide whether a submission is warranted.

Browser failure/sign-in does not lose the local export. Never describe native presentation as a completed upload or issue submission. `PRIVACY.md` documents local diagnostic collection, retention, deletion and user-initiated public sharing. Reference GitHub's [file attachment](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files) and [issue URL](https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/creating-an-issue) behavior; no GitHub API integration is needed.

### 6. Remove session-bound collection without rewriting supported data

Replace `Diagnostics::start(SessionLease, ...)` / ride-close ownership and the duplicate session-leased sink with the app-wide writer. Carry saved-session timing context as sanitized events, without persisting session identifiers or sample values. Preserve existing historical `session_diagnostic` tables, supported reader/validation paths and archive bytes; stop new duplicate session diagnostic writes. Do not remove the historical session writer/export API merely because the new app-wide owner no longer needs it if supported data/tests still rely on it.

There is no change to baseline 100, catalog/session SQL, archive format 4 or frozen fixtures. Diagnostic format versions are separate. A future incompatible diagnostic format requires explicit handling, not product reset authorization. Baseline import/reopen, existing recording exports and historical rows receive focused regression coverage.

## Risks / Trade-offs

- Strict allowlisting loses unrecognized firmware text and payload detail → retain command/extent/compatibility categories and use a separately designed opt-in protocol capture feature later if necessary.
- Process death can lose pending context → prompt background writes, atomic incident publication and explicit incomplete/loss reporting; do not claim this is a crash reporter.
- App-wide logging increases background I/O → aggregate successes, bounded queues/rotation and no blocking on telemetry or product-store locks; measure focused saturated-path behavior.
- Diagnostic storage and product storage can fail together → degrade to bounded memory context; surface export limitations honestly. Native service-construction failure remains outside this flow.
- Retention changes or clock anomalies can remove context → enforce independent byte/count limits and disclose oldest available boundary; users can export promptly.
- Preview handles expire on frontend recreation → incidents remain durable; the export button prepares a fresh report automatically.
- GitHub requires a manual attachment step → provide clear instructions, concise instructions and a broadly supported ZIP file rather than building an authenticated upload service.

## Migration Plan

1. Commit the completed planning artifacts before implementation, following repository OpenSpec phases.
2. Introduce the independent bounded store and typed event owner with deterministic clock/filesystem fault tests. Preserve product startup and baseline behavior.
3. Switch producers to the shared owner and remove superseded new session-bound writes; integrate contract/mock/server and both native adapters together in buildable slices.
4. Add immutable report preparation/export, frontend reporting UI, GitHub form and privacy/support docs. Validate supported archives unchanged.
5. Run focused shared/client/conformance/browser/native checks. Scope physical checks to save/share/system-browser presentation and affected lifecycle/file-protection behavior on iOS, macOS and Android; no new firmware/radio matrix is introduced. Record unavailable required checkpoints without claiming them.
6. Sync delta specs and archive only after implementation and relevant verification, in the final repository-prescribed commit. Rollback leaves baseline product data intact; independently owned diagnostic files are bounded and do not authorize resets or cleanup by older readers.
