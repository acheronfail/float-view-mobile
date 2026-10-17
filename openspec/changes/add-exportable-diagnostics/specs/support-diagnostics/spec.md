# Spec Delta

## Purpose

Let users preserve useful local failure context and deliberately share a bounded, sanitized diagnostic report through public GitHub issues.

## ADDED Requirements

### Requirement: Collection independent of recording

The process-owned service SHALL collect structured diagnostics before discovery and connection, independently of recording, tracking mode and disposable frontend lifetime. Diagnostic collection SHALL NOT create a saved board, ride, location record or session catalog entry.

#### Scenario: Initial identification fails

- **WHEN** a connection fails before firmware identification or session creation
- **THEN** its setup timeline and bounded failure reason are available for incident capture and export without a recording

#### Scenario: Guest or discarded provisional session

- **WHEN** a guest connection fails or an unretained saved-mode session is deleted
- **THEN** diagnostic incidents remain available under their own retention policy without retaining guest telemetry or the deleted ride

#### Scenario: Frontend recreation

- **WHEN** a WebView is recreated during a connection or capture
- **THEN** collection continues where OS execution is permitted and the replacement frontend can list existing incidents

### Requirement: Structured failure context

Reports SHALL include available app/build/channel, platform/OS/model, compatibility and configuration-schema facts, connection stages, request outcomes/timings, protocol failure categories, lifecycle/permission states, queue/loss metrics and storage/UI failure codes. Unknown facts SHALL remain unknown. Every event SHALL identify its process run, connection generation and operation when applicable.

#### Scenario: Unsupported firmware

- **WHEN** discovery observes unsupported firmware or an unaudited schema
- **THEN** the report includes bounded validated version/schema facts, the discovery stage and rejection category without fabricating motor or battery identity

#### Scenario: Reconnect and stale callback

- **WHEN** an old callback arrives after a replacement connection starts
- **THEN** diagnostics distinguish generations and identify the ignored callback without attributing it to the replacement controller

#### Scenario: Decode failure

- **WHEN** a response fails framing, CRC, extent, layout or value validation
- **THEN** diagnostics include available command kind, byte count and closed failure category without retaining response bytes or rejected values

### Requirement: Automatic and manual incidents

The service SHALL capture up to 30 seconds of available recent events and up to 10 seconds of subsequent events around detected connection, protocol, configuration, storage or UI failures and a manual problem mark. Reports SHALL disclose clipping, coalescing and interrupted capture. Repeated automatic failures SHALL be coalesced within 60 seconds; manual capture SHALL admit at most one new incident per 10 seconds.

#### Scenario: Undetected visible problem

- **WHEN** a user activates Mark a problem without a detected error
- **THEN** the service captures current bounded context and returns an incident identifier without requiring an error or active controller

#### Scenario: Error storm

- **WHEN** repeated failures occur within the automatic coalescing interval
- **THEN** occurrence counts and the latest reason are retained within bounds without allocating another incident for every failure

#### Scenario: Background or process interruption

- **WHEN** background suspension, shutdown or process death prevents completion of the subsequent-event window
- **THEN** committed context remains readable after restart and the incident reports its incomplete window without starting background execution solely to finish diagnostics

### Requirement: Bounded retention and nonblocking failure

Diagnostic journal and incident files SHALL occupy at most 8 MiB, retain at most three incidents of at most 1 MiB each, and expire records after seven days. In-memory recent context SHALL contain at most 256 events of at most 1,024 bytes each. Work queues, retries and waits SHALL have numeric limits. Saturation or diagnostic I/O failure SHALL NOT block Bluetooth, telemetry or product persistence.

#### Scenario: Retention limit

- **WHEN** a new incident exceeds the retained count or byte budget
- **THEN** oldest eligible diagnostic data is evicted deterministically and losses are disclosed without deleting product records or user-exported files

#### Scenario: Saturated or unavailable writer

- **WHEN** diagnostic storage is full, locked, protected or unavailable, or the queue fills
- **THEN** collection degrades boundedly, reports its availability/loss counters where possible, and does not recursively log its own failure

#### Scenario: Diagnostic corruption

- **WHEN** a diagnostic file is malformed or has an unsupported version
- **THEN** supported product data and the unrecognized file remain untouched, collection/export reports the limitation, and no compatibility reset occurs

### Requirement: Privacy by construction

Stored and exported diagnostics SHALL contain only allowlisted structured fields. Coordinates, routes, nicknames, Bluetooth addresses, private firmware/controller/BMS identifiers, stable board identities, tokens, arbitrary paths/strings, raw packets, full configurations and telemetry values SHALL be excluded. Exports SHALL use report-local aliases and relative event times without stable cross-report device identifiers.

#### Scenario: Adversarial private values

- **WHEN** device names, firmware text, OS errors, frontend errors or protocol responses contain private sentinel strings, coordinates, credentials or control characters
- **THEN** neither local diagnostic records, preview, ZIP, filename nor GitHub URL contains those values

#### Scenario: Useful hardware model

- **WHEN** a device or firmware reports arbitrary model text
- **THEN** only a recognized normalized model/version or bounded structural facts are retained; unrecognized text is represented as unknown

#### Scenario: Uncorrelated reports

- **WHEN** reports are prepared separately for the same controller
- **THEN** they contain no stable device alias or digest allowing device matching across reports

### Requirement: Immutable report preview and export

Users SHALL be able to prepare and preview an immutable sanitized report for a selected incident or current context, then export exactly that report as a ZIP of at most 5 MiB. The ZIP SHALL contain only `report.md`, `events.jsonl` and `manifest.json`, including diagnostic format version, report ID, capture bounds, omissions, loss counts and completeness. Full recording archives SHALL NOT be embedded.

#### Scenario: Context changes after preview

- **WHEN** new failures occur after a report is prepared
- **THEN** export uses the previewed report without silently adding subsequent events or another controller's context

#### Scenario: Report expired or cleared

- **WHEN** export names an expired, cleared or foreign prepared-report identifier
- **THEN** it fails with a stable bounded error and requires a new preview rather than substituting current context

#### Scenario: Export with product storage failure

- **WHEN** the service is running but session creation, catalog work or ride export fails
- **THEN** diagnostic preparation and staging can proceed through independent diagnostic storage if that storage is available

#### Scenario: Partial export or frontend loss

- **WHEN** preparation/export fails, is cancelled or loses its requesting frontend
- **THEN** partial staging is reclaimed within bounded limits; an already presented file remains valid until native completion or bounded abandonment

### Requirement: User controlled reporting and deletion

Settings and relevant error surfaces SHALL offer the same report flow on desktop and touch. Users SHALL see the included data categories, capture limitations and public-sharing notice before export, and SHALL be able to clear retained diagnostics and unpresented prepared reports without deleting product data. Clearing SHALL NOT revoke an active native file presentation or delete external copies.

#### Scenario: Primary pointer and keyboard activation

- **WHEN** a user activates a reporting control with a primary pointer or keyboard
- **THEN** the action occurs once; non-primary activation and cancelled gestures do not trigger it

#### Scenario: Clear while recording

- **WHEN** the user clears diagnostics during a ride
- **THEN** retained journal/incidents and unpresented prepared reports are cleared while recording, connections and product revisions remain unchanged; later events can begin new diagnostics

### Requirement: GitHub issue handoff

After preview, the flow SHALL let users save/share the ZIP and open the repository's GitHub bug-report form with a sanitized environment/error summary and report ID. It SHALL explain that issues and attachments are public and that ZIP attachment is manual. It SHALL NOT upload, submit issues, require app-held GitHub credentials or put diagnostic event streams in URLs.

#### Scenario: Create bug report

- **WHEN** the user chooses Open GitHub issue
- **THEN** the system browser opens the fixed project issue-form destination with the bounded reviewed summary; the user supplies reproduction steps, optional motor/battery details, attaches the saved ZIP and submits on GitHub

#### Scenario: Browser or sign-in unavailable

- **WHEN** the browser cannot open GitHub or the user cannot sign in
- **THEN** the exported ZIP and copyable summary remain usable and no diagnostic upload is attempted

### Requirement: Supported data compatibility

Diagnostic storage and report versions SHALL be independent of product storage baseline 100 and archive format 4. New collection SHALL NOT rewrite supported catalogs, session schemas, historical diagnostic rows or archives. Frozen baseline fixtures and existing imports SHALL remain supported without a reset or pre-baseline recovery path.

#### Scenario: Upgrade and reopen

- **WHEN** this feature opens a supported baseline catalog and imports a baseline recording
- **THEN** product records and historical diagnostic rows retain their existing semantics and only independently owned diagnostic files are added
