# Proposal

## Why

Controller, motor, firmware and platform combinations will exceed the hardware available for testing. Users need to preserve useful failure context, including failed initial connections, and share it through public GitHub issues without exposing private recordings or device identities.

## What Changes

- Extend the existing structured diagnostics into a bounded app-wide journal that starts with the shared service, independently of saved/guest mode and recording-session creation.
- Preserve recent context around detected failures and a manual “Mark a problem” action, retaining a small number of incidents across restarts.
- Record sanitized environment, discovered compatibility facts, connection/protocol/configuration failures, native lifecycle/queue metrics, storage failures and structured frontend errors. Represent unavailable hardware facts as unknown.
- Add a previewable, versioned diagnostic ZIP containing `report.md`, `events.jsonl` and `manifest.json`, with report-local aliases and explicit capture/loss limits. Exclude routes, device identifiers, packet payloads, configurations and full telemetry.
- Add one shared “Report a problem” flow from Settings and relevant error surfaces: select/capture an incident, preview, save/share, then open a prefilled GitHub bug-report form and attach the ZIP manually.
- Add mandatory authenticated diagnostic methods across the frontend, mock, shared server and both native host integrations; keep diagnostic history out of complete application snapshots.
- Replace the recording-session dependency and duplicate capture sink for new diagnostic collection, retaining supported session schemas and historical archive contents.

Non-goals: automatic uploads, a developer telemetry service, GitHub authentication inside the app, automatic issue submission or attachment upload, full crash-dump collection, startup-recovery UI when the shared service cannot start, arbitrary packet capture, raw exception strings/stacks, full configuration export, or an exhaustive hardware compatibility matrix. A separate opt-in detailed protocol capture mode is deferred.

## Capabilities

### New Capabilities

- `support-diagnostics`: Local bounded incident collection, privacy-safe report preparation and export, and a user-controlled GitHub reporting flow.

### Modified Capabilities

- `backend-contract`: Mandatory diagnostic requests and stable results/errors; distinguish app-wide incident correlation from single-controller recording ownership.

## Impact

- Shared Rust `backend-service`, `loopback-server`, runtime diagnostic event boundaries and bounded file workers; existing Apple C and Android JNI bindings and native environment/file-presentation adapters.
- Frontend contract/client/mock, Settings/error reporting UI and structured frontend failure reporting; canonical conformance scenarios and focused browser/native integration coverage.
- New independently versioned, bounded private diagnostic storage and export staging. No catalog/session schema change, owned-product-storage marker change, archive-format change or rewriting of baseline data. Diagnostic failure must never reset product storage.
- Replace new writes through the session-leased diagnostic sink; keep existing supported diagnostic tables and archive readers intact. Reuse existing metrics and file-presentation machinery rather than creating another runtime or native product service.
- Add `.github/ISSUE_TEMPLATE/bug_report.yml`; update privacy/support documentation. Prefer existing libraries; any ZIP dependency must replace hand-written archive complexity and have a bounded feature set.
