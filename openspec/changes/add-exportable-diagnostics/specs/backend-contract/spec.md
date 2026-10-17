# Spec Delta

## ADDED Requirements

### Requirement: Mandatory diagnostic operations

Every supported backend SHALL implement authenticated `diagnostics.list`, `diagnostics.capture`, `diagnostics.prepare`, `diagnostics.export`, `diagnostics.clear` and `diagnostics.uiError` with identical semantics. Existing `diagnostics.report` timing ingestion SHALL remain. Diagnostic content SHALL remain outside `AppState`; operations SHALL NOT change product, settings or session catalog revisions.

#### Scenario: Supported host and mock

- **WHEN** a bundled frontend uses a supported Apple, Android or mock backend
- **THEN** every diagnostic method is available through the existing authenticated request/result/error envelopes without optional-method negotiation

#### Scenario: Invalid authentication or parameters

- **WHEN** a diagnostic operation has invalid authentication, unknown fields, oversized values or malformed identifiers
- **THEN** it is rejected before diagnostic file lookup or side effects using existing authentication and stable error semantics

### Requirement: Diagnostic incident discovery and capture

`diagnostics.list` SHALL accept empty parameters and return availability/loss status and at most three incident summaries. `diagnostics.capture` SHALL accept empty parameters and return a bounded incident ID and capture status. Summaries SHALL contain only opaque IDs, closed reasons, completeness and relative age. Capture SHALL not wait for its subsequent-event window to finish.

#### Scenario: Empty or unavailable storage

- **WHEN** no incident exists or diagnostic persistence is unavailable
- **THEN** listing succeeds with an empty or available bounded list and explicit persistence status rather than implying that no error occurred

#### Scenario: Capture under rate limit

- **WHEN** another manual capture is requested within ten seconds
- **THEN** it returns `unavailable` without allocating another incident or delaying connection work

### Requirement: Prepared diagnostic report contract

`diagnostics.prepare` SHALL accept a nullable incident ID, with null selecting current context, and return an opaque report ID, sanitized Markdown summary, inclusion/completeness metadata and expiry. The immutable report SHALL be client-owned, expire after ten minutes, and obey a global capacity of two prepared reports. Preview results SHALL occupy at most 32,768 UTF-8 bytes; summary text at most 8,192 bytes.

#### Scenario: Prepare incomplete incident

- **WHEN** an incident is still capturing or has an interrupted subsequent-event window
- **THEN** preparation returns an immutable snapshot disclosing its captured boundary and missing window without waiting for another ten seconds

#### Scenario: Capacity and unknown incident

- **WHEN** report capacity is exhausted or the requested incident no longer exists
- **THEN** preparation returns `unavailable` or `not_found` respectively without evicting a live prepared report or selecting a different incident

### Requirement: Diagnostic export and clear contract

`diagnostics.export` SHALL accept only a prepared-report ID and return the existing bounded `ExportResult` after native presentation/staging. `diagnostics.clear` SHALL accept empty parameters and return `Accepted` after clearing retained diagnostics and invalidating unpresented reports. No method SHALL accept filesystem paths or return archive bytes in control messages.

#### Scenario: Expired or foreign report

- **WHEN** export names a report owned by another client, cleared, expired or lost on process restart
- **THEN** it returns `not_found` without exporting replacement content

#### Scenario: Frontend disconnect during presentation

- **WHEN** the frontend disconnects after native file presentation starts
- **THEN** existing correlated file lease semantics preserve the file until completion or bounded abandonment and reject stale completions

#### Scenario: Clear during capture

- **WHEN** clearing races with queued diagnostic writes, capture finalization or report preparation
- **THEN** a new diagnostic generation invalidates the old work so it cannot repopulate cleared records or reports; active native presentation leases remain valid

### Requirement: Structured frontend error ingestion

`diagnostics.uiError` SHALL accept only a closed error category, closed feature identifier and optional bounded numeric source positions defined in the contract, and return `Accepted`. Raw messages, stacks, URLs, DOM text and arbitrary objects SHALL be rejected. Ingestion SHALL be rate limited to ten reports per minute per client with bounded aggregate service admission and no error-reporting recursion.

#### Scenario: Error contains user content

- **WHEN** a frontend exception includes a board name, route, URL, token or private text
- **THEN** the frontend sends only the mapped category/feature/source positions and those private strings never enter the diagnostic request

#### Scenario: Reporting itself fails

- **WHEN** diagnostic ingestion is unavailable, rate limited or disconnected
- **THEN** the frontend handles that failure without another diagnostic submission and continues normal recovery

## MODIFIED Requirements

### Requirement: One controller per telemetry session

Live telemetry, transition events, durable samples, summaries, session-associated diagnostics, and recording exports SHALL belong to exactly one selected direct-or-CAN controller route at a time. Selecting or validating a different controller SHALL end the existing session and clear route-scoped telemetry before values from the new controller are accepted. App-wide diagnostic reports SHALL distinguish process runs, connection generations and report-local controller aliases, and SHALL NOT merge observations from different controllers into one operation or sample stream.

#### Scenario: Selected controller changes

- **WHEN** the active selection changes from a direct controller to a CAN-routed controller
- **THEN** the prior session ends before the first new-controller sample and no session metadata/data or recording export combines values from both controllers

#### Scenario: Incident spans replacement connection

- **WHEN** an app-wide incident includes a failed controller attempt and a subsequent connection to another controller
- **THEN** events identify distinct generations and aliases without changing single-controller recording ownership
