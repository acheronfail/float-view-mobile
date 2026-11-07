## MODIFIED Requirements

### Requirement: Bounded retention and nonblocking failure

Diagnostic journal and incident files SHALL occupy at most 8 MiB, retain at most ten incidents of at most 1 MiB each, and expire records after seven days. In-memory recent context SHALL contain at most 256 events of at most 1,024 bytes each. Work queues, retries and waits SHALL have numeric limits. Saturation or diagnostic I/O failure SHALL NOT block Bluetooth, telemetry or product persistence.

#### Scenario: Retention limit

- **WHEN** a new incident exceeds the retained count or byte budget
- **THEN** oldest eligible diagnostic data is evicted deterministically and losses are disclosed without deleting product records or user-exported files

#### Scenario: Saturated or unavailable writer

- **WHEN** diagnostic storage is full, locked, protected or unavailable, or the queue fills
- **THEN** collection degrades boundedly, reports its availability/loss counters where possible, and does not recursively log its own failure

#### Scenario: Diagnostic corruption

- **WHEN** a diagnostic file is malformed or has an unsupported version
- **THEN** supported product data and the unrecognized file remain untouched, collection/export reports the limitation, and no compatibility reset occurs

#### Scenario: Ten incident retention across restarts

- **WHEN** ten incidents fit within the existing byte budget and the app restarts
- **THEN** all ten remain available until expiry or eviction, and an eleventh evicts the oldest eligible incident

### Requirement: Privacy by construction

Stored and exported diagnostics SHALL contain only bounded structured fields and classified diagnostic content. Configuration contents, telemetry values, schema fingerprints/signatures and sanitized protocol bytes SHALL be permitted. Coordinates, travelled routes, device identities, nicknames, credentials, paths and arbitrary firmware/error text SHALL remain excluded. Exports SHALL use report-local correlation and relative event times. Nothing SHALL be uploaded automatically.

#### Scenario: Adversarial private values

- **WHEN** device names, firmware text, OS errors, frontend errors or identity/location-bearing protocol responses contain private sentinel strings, coordinates, credentials or control characters
- **THEN** neither local diagnostic records, prepared summary, ZIP, filename nor GitHub URL contains those values

#### Scenario: Useful hardware model

- **WHEN** a device or firmware reports arbitrary model text
- **THEN** only a recognized normalized model/version or bounded structural facts are retained; unrecognized text is represented as unknown

#### Scenario: Uncorrelated reports

- **WHEN** reports are prepared separately for the same controller
- **THEN** they contain no stable device alias or digest allowing device matching across reports

#### Scenario: Authorized technical content

- **WHEN** configuration work or telemetry exchanges are captured
- **THEN** configuration values and actual schema identities, decoded non-location telemetry and classified packet bytes are available within unchanged event/report bounds with explicit fragment offsets and truncation disclosure

#### Scenario: Packet privacy boundary

- **WHEN** packets contain routing wrappers, firmware identities, location fields or unclassified contents
- **THEN** excluded fields are redacted or that raw packet is omitted with a closed reason; arbitrary unknown bytes are never dumped, and available decoded non-location telemetry remains eligible

### Requirement: Supported data compatibility

Diagnostic storage and report versions SHALL be independent of product storage baseline 100 and archive format 4. New collection SHALL NOT rewrite supported catalogs, session schemas, historical diagnostic rows or archives. Frozen baseline fixtures and existing imports SHALL remain supported without a reset or pre-baseline recovery path.

#### Scenario: Upgrade and reopen

- **WHEN** this feature opens a supported baseline catalog and imports a baseline recording
- **THEN** product records and historical diagnostic rows retain their existing semantics and independently owned diagnostic storage remains separate from product records

#### Scenario: Restartable diagnostic format upgrade

- **WHEN** supported version-1 diagnostics are opened by the version-2 collector or an upgrade is interrupted
- **THEN** existing incident and journal contents are preserved through an explicit restartable marker transition; unknown markers, corrupt data and operational errors preserve files and fail actionably

### Requirement: Explain configuration operation failures

Configuration and control diagnostics SHALL retain admission, completion and rejection context: failure layer/category, authoritative write blocker, readiness, pending work, revision and identity match facts, and transport/application correlation when available. Context SHALL be captured at the operation boundary with permitted technical contents, excluding device identities and raw errors. Public error behavior and mutation authorization SHALL remain unchanged.

#### Scenario: Rejected configuration write

- **WHEN** a write is rejected by readiness, safety, pending work, identity or revision checks
- **THEN** diagnostics distinguish those conditions using closed codes and structural facts from that attempt, including missing or unknown facts

#### Scenario: Accepted operation fails later

- **WHEN** admitted configuration work fails during scheduling, acknowledgement, readback, timeout or connection replacement
- **THEN** diagnostics retain its bounded correlation, available failure category and completion outcome so rejection can be distinguished from a failed accepted operation

#### Scenario: Coalesced late rejection

- **WHEN** a rejection joins an incident after its subsequent-event window closes
- **THEN** its failure context remains in the exported incident within existing event and byte bounds, without reopening the window

#### Scenario: Private request inputs

- **WHEN** device identities, credentials or unrelated unrecognized input fields contain private text
- **THEN** failure records and exported ZIPs retain permitted technical content while preserving the remaining privacy exclusions

#### Scenario: Audited prerelease context

- **WHEN** a connected controller reports an audited stable, beta2 or beta3 Refloat release
- **THEN** diagnostic compatibility context retains numeric version components and its closed release suffix without accepting arbitrary firmware text

#### Scenario: Observed schema rejected for admission

- **WHEN** a structurally valid schema fails audited configuration admission
- **THEN** diagnostics retain its observed fingerprint and wire signature without granting write authority or retaining untrusted schema text

### Requirement: Immutable report preparation and export

The export flow SHALL automatically prepare an immutable sanitized report for the newest retained incident or current context when none exists, then export exactly that report as a ZIP of at most 5 MiB. The ZIP SHALL contain only `report.md`, `events.jsonl` and `manifest.json`, including diagnostic format version, report ID, capture bounds, omissions, loss counts and completeness. Full recording archives SHALL NOT be embedded.

#### Scenario: Context changes after preparation

- **WHEN** new failures occur after a report is prepared
- **THEN** export uses the prepared report without silently adding subsequent events or another controller's context

#### Scenario: Report expired or cleared

- **WHEN** export names an expired, cleared or foreign prepared-report identifier
- **THEN** it fails with a stable bounded error and requires fresh preparation rather than substituting current context

#### Scenario: Export with product storage failure

- **WHEN** the service is running but session creation, catalog work or ride export fails
- **THEN** diagnostic preparation and staging can proceed through independent diagnostic storage if that storage is available

#### Scenario: Partial export or frontend loss

- **WHEN** preparation/export fails, is cancelled or loses its requesting frontend
- **THEN** partial staging is reclaimed within bounded limits; an already presented file remains valid until native completion or bounded abandonment

#### Scenario: Current context contains configuration contents

- **WHEN** a current-context export is prepared while configuration is available and no error incident was captured
- **THEN** the immutable report includes the current permitted configuration contents and schema identity without creating an incident or changing product state
