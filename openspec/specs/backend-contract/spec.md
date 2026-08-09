# Backend Contract Specification

## Purpose

Define the complete platform-neutral contract implemented by every supported native backend and consumed by the shared frontend.

## Requirements

### Requirement: Complete mandatory contract

Every supported backend SHALL implement the WebSocket control methods `state.get`, `bluetooth.requestPermission`, `bluetooth.scan`, `device.connect`, `device.selectController`, `device.disconnect`, `controller.shutdown`, `refloat.control.set`, `refloat.fatalError.clear`, `savedDevice.update`, `savedDevice.delete`, `settings.update`, `boardProfile.get`, `boardProfile.update`, `session.list`, `session.get`, `session.export`, `session.delete`, `session.delete.cancel`, `session.prune.preview`, `session.prune.start`, and `session.prune.cancel`, plus the authenticated session-data request kinds `series`, `samples`, and `events`. Session-group methods SHALL NOT remain; `session.getDiagnostics` SHALL NOT remain; exact retained historical reads use `samples`. The frontend MUST NOT negotiate or branch on optional methods or platform capabilities.

#### Scenario: Supported backend

- **WHEN** Android, iOS, or macOS is declared supported
- **THEN** every mandatory control method and session-data request kind is available with the same request, result, event, and error semantics

#### Scenario: Unsupported method

- **WHEN** a request names a method or session-data kind outside the mandatory contract
- **THEN** the backend returns an `invalid_request` error

### Requirement: Authenticated loopback transport

The backend SHALL bind only to loopback and expose `/`, `/api/bootstrap`, `/api/ws`, and the session-data URL identified by bootstrap. Bootstrap SHALL provide the WebSocket URL, session-data URL, and one random per-launch token. The WebSocket SHALL validate the token and expected Origin. Every session-data request SHALL be an Origin-bearing POST, SHALL validate the same token from `X-Float-View-Token` and the exact expected Origin before reading the bounded body or resolving a session, and SHALL return `Cache-Control: no-store` without placing the token in a URL, response, error, or log. Browser development SHALL resolve both data transports through the authenticated development origin rather than introducing cross-origin behavior.

#### Scenario: Valid frontend connection

- **WHEN** the bundled frontend uses the current bootstrap token and application Origin
- **THEN** the backend accepts its WebSocket and bounded session-data requests

#### Scenario: Invalid authentication

- **WHEN** a WebSocket or session-data request omits or supplies an incorrect token or Origin
- **THEN** the backend rejects it before body parsing or session lookup and discloses neither session existence nor private data

#### Scenario: Recreated frontend

- **WHEN** the frontend is recreated during the same native launch
- **THEN** bootstrap provides the current authenticated control-plane and session-data endpoints without exposing durable state through an unauthenticated route

### Requirement: Bounded request protocol

WebSocket control requests SHALL use `{ id, method, params }`; successes SHALL use `{ id, result }`; and failures SHALL use `{ id, error }`. Session-data POST bodies and results SHALL use the bounded discriminated shapes defined for `series`, `samples`, and `events`, with bounded errors using the same frontend-visible error vocabulary. Every accepted WebSocket request ID and every accepted HTTP request MUST receive exactly one terminal result unless its client connection is cancelled, and IDs, messages, bodies, decoded responses, queues, timeouts, retries, cursors, and collections MUST be bounded.

#### Scenario: Successful request

- **WHEN** a valid control or session-data request is processed
- **THEN** exactly one corresponding bounded result is returned

#### Scenario: Failed request

- **WHEN** a request cannot be completed
- **THEN** exactly one structured bounded error is returned without partial success data

#### Scenario: Oversized message

- **WHEN** a client exceeds the documented WebSocket, HTTP body, decoded response, field, point, page, or cursor bound
- **THEN** the backend rejects the request or connection before unbounded allocation or storage work

### Requirement: Stable errors

The contract SHALL use only `invalid_request`, `invalid_state`, `permission_required`, `not_found`, `conflict`, `unavailable`, and `internal` as frontend-visible error codes.

#### Scenario: Permission required

- **WHEN** scanning requires an unresolved OS permission
- **THEN** `bluetooth.scan` fails with `permission_required`

#### Scenario: Revision conflict

- **WHEN** settings, a saved-device custom name, or a Board Profile is updated with a stale expected revision
- **THEN** the request fails with `conflict` and persisted data is unchanged

### Requirement: Authoritative application state

`state.get` SHALL return a bounded `AppState` containing a monotonic revision, a separate monotonic `sessionCatalogRevision`, Bluetooth state, discovered devices with an explicit saved-board match when known, saved devices with canonical VESC UUID identity when meaningful, resolved identity, and optional local platform association, active connection with `trackingMode` (`saved` or `guest`) and nullable saved-device identity, connection state, detected direct-or-CAN controller route, bounded controller candidates with canonical UUIDs when selection is required, bounded connection issue when failed, firmware including canonical UUID when meaningful, complete live-only Refloat control state when protocol discovery has begun, complete bounded controller-action state, BMS identity and state, saved-mode active-session summary when present, the latest successfully persisted sample for saved mode or bounded ephemeral latest sample for guest mode when known, and settings. Recent session summaries and session groups SHALL NOT be included; standalone or board-filtered session summaries SHALL be retrieved through `session.list`. Historical series and diagnostic windows MUST NOT be included.

#### Scenario: Initial state

- **WHEN** the frontend requests `state.get` after connecting
- **THEN** it receives all bounded state needed to render the main application, including saved-board matches, tracking mode, and current Refloat control availability when active, with no session-summary or session-group collection

#### Scenario: Guest state

- **WHEN** a guest connection publishes a live sample
- **THEN** the authoritative snapshot may expose that bounded ephemeral sample and live-only Refloat controls for LiveView while active, but exposes no saved-device identity, active durable session, or catalog entry for it

#### Scenario: Catalog revision is independent

- **WHEN** a persisted saved-mode telemetry sample or confirmed Refloat control publishes a newer complete state snapshot without changing session catalog membership or summary visibility
- **THEN** the global state revision advances while `sessionCatalogRevision` remains unchanged

#### Scenario: Recreated frontend during reconnect

- **WHEN** a disposable frontend reconnects while a saved or guest board is reconnecting
- **THEN** initial authoritative state contains the current tracking mode, connection state, detected route when known, pending candidates or connection issue when applicable, the latest permitted sample when one exists, Refloat controls in a non-actionable loading state when discovery had begun, and the current session catalog revision

### Requirement: Authoritative state events

Every frontend-visible coarse state change, every successfully persisted saved-mode active-session sample, and every bounded guest live-sample update intended for LiveView SHALL increment the state revision and publish `{ event: "state.changed", data: AppState }`. Saved-mode sample-driven events SHALL occur at the configured phase-appropriate persistence cadence; guest sample-driven events SHALL use the same bounded live update cadence without creating catalog-visible persistence. The backend SHALL conflate pending state events to the newest complete snapshot rather than allowing an unbounded queue or preserving an older pending snapshot.

#### Scenario: Connection transition

- **WHEN** an active saved or guest connection changes state
- **THEN** the frontend receives a newer complete `AppState`

#### Scenario: Persisted saved live sample

- **WHEN** the native persistence gate selects a saved-mode telemetry sample and its durable append succeeds
- **THEN** the frontend receives a newer complete state containing that sample and resulting active-session aggregates

#### Scenario: Ephemeral guest sample

- **WHEN** a guest telemetry sample is available for live display
- **THEN** the frontend receives a bounded state update containing it without a durable session append or catalog revision change

#### Scenario: Skipped probe

- **WHEN** a decoded telemetry probe is not selected for saved persistence or guest live publication
- **THEN** that probe does not cause a frontend state event

#### Scenario: Event ordering

- **WHEN** multiple changes occur while an event is pending
- **THEN** the eventual event contains the newest authoritative revision and stale revisions do not replace newer frontend state

#### Scenario: Slow frontend

- **WHEN** frontend consumption is slower than native sample production
- **THEN** pending delivery converges on the newest complete snapshot without delaying durable saved-mode persistence or growing an unbounded queue

### Requirement: Backend-owned Bluetooth operations

The backend SHALL own permission resolution, bounded scanning, saved-board matching, stable logical identity derivation, explicit saved-or-guest connection intent, BLE stream setup, connection, controller identification and selection validation, reconnect, disconnection, protocol polling, and firmware discovery. Exactly zero or one BLE device MAY be active. A connection generation SHALL retain its tracking mode for retries and lifecycle recovery. Only a Home discovery result may initiate a connection; saved-board history and configuration routes SHALL not start Bluetooth work.

#### Scenario: Bounded scan

- **WHEN** the frontend requests `bluetooth.scan`
- **THEN** the backend starts discovery, stops it within the contract bound, and publishes discovered devices with exact saved-board match information when available

#### Scenario: Competing connection

- **WHEN** a second device is requested while another device is active
- **THEN** the backend returns `invalid_state` and preserves the existing connection

#### Scenario: Known saved connection

- **WHEN** `device.connect` requests a discovered device that matches a saved board in saved mode
- **THEN** the backend accepts the saved connection and does not require a second save decision

#### Scenario: Unknown guest connection

- **WHEN** `device.connect` requests an unknown device in guest mode
- **THEN** the backend accepts the bounded connection without creating a saved-device record or reading a Board Profile as a durable write source

#### Scenario: Retry failed device

- **WHEN** `device.connect` names the same active device while its state is `failed`
- **THEN** the backend accepts a new bounded connection generation using the retained tracking mode, using the latest Board Profile only for saved mode

#### Scenario: WebView recreation

- **WHEN** the frontend WebView is destroyed and recreated
- **THEN** the native connection, tracking mode, identification, pending controller selection, and backend jobs continue where the OS permits and the new frontend recovers them through `state.get`

### Requirement: Backend-owned durable sessions

A ready saved-mode connection SHALL automatically create one provisional durable session, and all saved-mode live display, review data reads, and persistence SHALL derive from one sequenced native sample pipeline. The latest saved-mode sample SHALL become frontend-visible only after the same sample is selected and successfully appended to that active session. A ready guest connection SHALL create no durable session and SHALL use a bounded in-memory live sample path. When a saved-mode session closes or is recovered after interruption, the backend SHALL retain it only if at least one persisted sample entered the `riding` state and its accumulated distance is greater than or equal to `minimumSessionDistance`; otherwise it SHALL delete the catalog entry and session files. Guest samples, locations, summaries, and ride metadata SHALL never enter that retention decision.

#### Scenario: Ready saved board

- **WHEN** a supported saved-mode board becomes ready
- **THEN** the backend starts a provisional durable session without a frontend logging command

#### Scenario: Ready guest board

- **WHEN** a supported guest-mode board becomes ready
- **THEN** the backend starts live telemetry without creating a session, saved-device record, catalog row, or durable ride log

#### Scenario: Idle-only saved connection

- **WHEN** a provisional saved-mode session closes without any persisted sample entering `riding`
- **THEN** the backend deletes the session even if idle telemetry accumulated distance

#### Scenario: Ride below the configured minimum

- **WHEN** a provisional saved-mode session entered `riding` but closes below `minimumSessionDistance`
- **THEN** the backend deletes the session instead of exposing it in session history

#### Scenario: Live sample ordering

- **WHEN** a saved-mode telemetry sample is selected for persistence
- **THEN** its append completes before the state revision exposes that sample and updated session summary

#### Scenario: Guest sample ordering

- **WHEN** a guest telemetry sample is selected for live publication
- **THEN** the state update exposes only the bounded ephemeral sample and no durable append, location write, catalog revision, or session summary is produced

#### Scenario: Unexpected disconnect

- **WHEN** a saved or guest BLE connection ends unexpectedly
- **THEN** saved mode retains the last persisted live sample while reconnecting and closes or recovers the session deterministically, while guest mode retains only bounded in-memory live state and reconnects without creating history

#### Scenario: Active connection removed

- **WHEN** the backend removes or replaces the active connection
- **THEN** its retained live sample, tracking mode, and active-session association are cleared; no guest state is promoted to saved history

### Requirement: Current frontend product operations

The contract SHALL support revision-checked saved-device custom-name updates, persisted settings including nullable `historyGapThresholdSeconds`, local versioned Board Profiles, bounded cursor-paginated standalone session summaries, small session metadata retrieval, authenticated bounded historical-data queries, native export presentation, irreversible ride pruning, and deletion with platform-neutral semantics. Session grouping, automatic linking, linked-ride state, and group mutation operations SHALL NOT be part of the contract. A configured history-gap threshold SHALL be finite and from 0.1 through 3,600 seconds; null or absence SHALL select the derived default. `session.get` SHALL return the requested summary, ordered available numeric-field descriptors, event count, recorded/stored sample counts, prune metadata, and one published snapshot containing history revision, committed sample sequence, committed event sequence, and committed elapsed boundary, but SHALL NOT contain historical numeric points or transition collections. Historical overview series, exact or retained sample pages, and event pages SHALL use the session-data endpoint and a snapshot returned by `session.get`.

#### Scenario: Saved board rename

- **WHEN** the frontend supplies a valid custom display name or clears the override at the current saved-device revision
- **THEN** the backend persists it and returns saved-device state with a newer revision and newly resolved display name

#### Scenario: Session review metadata

- **WHEN** the frontend requests `session.get`
- **THEN** the backend returns bounded metadata and one published snapshot in a message no larger than 65,536 UTF-8 bytes without embedding historical series or events

#### Scenario: Session catalog page

- **WHEN** the frontend requests a valid standalone session-list page with a bounded limit and optional cursor
- **THEN** the backend returns summaries ordered by `startedAt` descending with `sessionId` descending tie-breaking, plus an opaque continuation cursor when more rows remain

#### Scenario: Grouping is absent

- **WHEN** the frontend requests state, settings, or supported control methods
- **THEN** no session-group collection, automatic-linking option, linked-ride operation, or group persistence result is exposed

#### Scenario: Historical data review

- **WHEN** the frontend submits an authenticated series, samples, or events request for the returned snapshot
- **THEN** the backend returns the corresponding bounded committed history with the same platform-neutral names, units, nullability, and sequence ordering

#### Scenario: Configure history gaps

- **WHEN** the frontend persists a valid history-gap threshold in seconds or clears it to null
- **THEN** complete application state reports that value and subsequent series requests use the configured threshold or the defined derived default without platform branching

#### Scenario: Diagnostic review

- **WHEN** the frontend submits a samples request for an exact bounded window around a valid elapsed time
- **THEN** the session-data endpoint returns exact pageable retained source samples without creating a separate logging or WebSocket method, equal to the original lossless stream only when the ride has not been pruned

#### Scenario: Native export

- **WHEN** the frontend requests a supported session export
- **THEN** the backend creates the export and invokes the host platform save or share presentation without frontend platform branching, disclosing any pruning policy and recorded-versus-stored counts

#### Scenario: Cancel pending deletion

- **WHEN** the user cancels deletion while it is still waiting for reader leases and before file removal begins
- **THEN** pending deletion clears and the unchanged session becomes readable again; if removal already won, the backend reports the stable completed-deletion result

### Requirement: Disposable current storage schema

The native backends SHALL execute the shared canonical first-production SQLite schemas, retain WAL recovery and integrity handling, and recreate all incompatible test-stage storage on schema-version mismatch rather than running migrations. The session schema SHALL use dedicated nullable columns for every stable numeric telemetry field, bounded JSON only for negotiated numeric extensions, bounded write-time numeric-field presence metadata, chronological transition rows linked to source samples, elapsed indexes, incremental auto-vacuum, session-captured riding/idle persistence intervals, history/prune metadata, and one authoritative commit marker containing committed sample/event boundaries, recorded/stored counts, history revision, elapsed coverage, and the bounded riding/distance/maximum/source-sequence/last-value projection needed to recover catalog summaries. The catalog SHALL contain bounded persisted prune-job progress. Denormalized catalog metadata SHALL be reconciled from the authoritative session database after interruption or an in-process catalog failure before that session is published or appended again.

#### Scenario: Schema mismatch

- **WHEN** the canonical storage version differs from persisted pre-release test data
- **THEN** the backend deletes its logging directory and recreates the complete current schema without migration, backfill, compatibility columns, or partial fallback

#### Scenario: Interrupted session recovery

- **WHEN** the backend starts with an interrupted active session whose session marker is newer than its catalog row
- **THEN** it performs bounded integrity recovery, reconciles published boundaries and counts from the session database, and only then exposes the recovered result

#### Scenario: Catalog update fails after session commit

- **WHEN** a session transaction commits but its denormalized catalog update fails in the running process
- **THEN** the backend marks that session dirty, publishes no newer boundary, and reconciles from the marker before accepting another append or metadata read

#### Scenario: Late extension field

- **WHEN** a valid extension field first appears after the first 2,000 committed samples
- **THEN** snapshot metadata includes it through write-time field presence without scanning or sampling the ride

### Requirement: Complete persisted settings shape

The mandatory `Settings` value SHALL contain `revision`, `idleProbeIntervalMs`, `idlePersistIntervalMs`, `ridingPersistIntervalMs`, `postRideTailMs`, `reconnectDurationMs`, `minimumSessionDistance`, `themePreference`, and `unitPreference`. `minimumSessionDistance` SHALL be expressed in meters and default to `100`; `themePreference` SHALL accept exactly `system`, `light`, or `dark`; `unitPreference` SHALL accept exactly `metric` or `imperial` and default to `metric`; `automaticSessionLinkingEnabled`, `sessionLinkWindowMs`, and `rawPacketCaptureEnabled` MUST NOT remain in the contract or persisted settings payload.

#### Scenario: Default settings

- **WHEN** a backend creates current default settings
- **THEN** it returns the existing timing defaults with `minimumSessionDistance` equal to `100`, `themePreference` equal to `system`, `unitPreference` equal to `metric`, and no linking or raw-packet fields

#### Scenario: Settings round trip

- **WHEN** the frontend updates a valid appearance, units, timing, or distance field at the current revision
- **THEN** every supported backend persists and returns the same complete settings shape at a newer revision

#### Scenario: Removed linking option

- **WHEN** a settings patch contains `automaticSessionLinkingEnabled` or `sessionLinkWindowMs`
- **THEN** every supported backend rejects the patch and leaves persisted settings unchanged

#### Scenario: Unknown settings field

- **WHEN** a settings patch contains `rawPacketCaptureEnabled` or another field outside the complete shape
- **THEN** every supported backend rejects the patch and leaves persisted settings unchanged

### Requirement: Explicit live sample shape

The contract SHALL define one bounded latest live sample with sequence, elapsed time, ride state, board state, footpad state, pack voltage, duty cycle, controller temperature, motor temperature, motor current, battery current, eRPM, and speed. Every field SHALL have fixed platform-neutral semantics and the contract MUST NOT expose an unbounded or catch-all live field map.

#### Scenario: Complete supported telemetry

- **WHEN** either native backend serializes a successfully persisted telemetry sample
- **THEN** it supplies the same complete named live sample shape with unavailable nullable readings represented as null

#### Scenario: New telemetry

- **WHEN** support for another live telemetry field is introduced
- **THEN** the TypeScript contract, both native backends, persistence where applicable, conformance, and frontend placement change together

### Requirement: Consistent saved-board identity

Saved-device state SHALL distinguish a nullable custom display-name override, advertised Bluetooth name, optional local platform Bluetooth identifier, canonical VESC UUID identity when meaningful, resolved display name, and mutation revision. For a meaningful local VESC endpoint firmware identity, `savedDeviceId` and `deviceHistoryId` SHALL both equal its 24-character lowercase hexadecimal UUID; `deviceHistoryId` SHALL remain stable in session archives and SHALL associate imported sessions with a board. When no meaningful UUID exists, native code SHALL use bounded random identifiers and SHALL not claim an automatic cross-installation match. The resolved display name SHALL use the first non-empty value from custom name, advertised name, local platform identifier, and stable logical identity in that order. Apple platform identifiers SHALL use the CoreBluetooth peripheral UUID and Android SHALL use the Bluetooth address, matching VESC Tool semantics without frontend platform branching. Imported boards MAY have no local platform identifier and SHALL remain valid saved-board records.

#### Scenario: Custom board name

- **WHEN** a saved board has a non-empty custom name
- **THEN** saved-board and active-connection state use that value as the resolved display name

#### Scenario: Advertised-name fallback

- **WHEN** no custom name exists and the board has a non-empty advertised Bluetooth name
- **THEN** that advertised name is the resolved display name

#### Scenario: Platform-identifier fallback

- **WHEN** neither custom nor advertised name exists and a local platform identifier is present
- **THEN** the opaque local platform identifier is the resolved display name

#### Scenario: Stable-identity fallback

- **WHEN** an imported board has no custom name, advertised name, or local platform identifier
- **THEN** its bounded stable logical identity is the resolved display name and the board remains configurable

#### Scenario: Rediscovery after rename

- **WHEN** a custom-named saved board is rediscovered with an updated advertised name
- **THEN** the backend refreshes advertised identity without replacing the custom-name override

#### Scenario: Archive identity merge

- **WHEN** a session archive is imported with a `deviceHistoryId` already present locally
- **THEN** the backend reuses that saved board and attaches imported sessions to it without creating a duplicate

#### Scenario: Identifier disclosure boundary

- **WHEN** board state is rendered or ordinary diagnostics and exports are produced
- **THEN** local platform identifiers are absent from ordinary logs and portable archives while canonical VESC firmware/controller/BMS UUIDs may be retained as application and portable board metadata

### Requirement: Controller connection state shape

`DeviceState` SHALL include `identifying` and `selecting_controller`. `ActiveConnection` SHALL contain `controllerConnection`, which is null before a route is verified in the current generation or is exactly `{ kind: "direct" }` or `{ kind: "can", canId }`; a `controllerCandidates` array bounded to 16; and nullable `connectionIssue`. A ready connection MUST have a non-null route, supported controller and Refloat identity, an established telemetry plan, an empty candidate array, and no issue.

#### Scenario: Direct route becomes ready

- **WHEN** identification completes for a supported direct controller
- **THEN** authoritative state reports `ready`, `{ kind: "direct" }`, complete firmware identity, and no candidates

#### Scenario: CAN route becomes ready

- **WHEN** identification completes for supported controller CAN ID `42`
- **THEN** authoritative state reports `ready`, `{ kind: "can", canId: 42 }`, complete firmware identity, and no candidates

#### Scenario: Identification is incomplete

- **WHEN** the BLE link exists but controller route or compatibility negotiation is incomplete
- **THEN** state is not `ready` and no durable session or telemetry polling starts

#### Scenario: New reconnect generation

- **WHEN** reconnect begins after a verified route
- **THEN** the public route, candidates, and issue are cleared until that generation verifies or fails them while the latest persisted sample remains retained

### Requirement: Controller candidate and selection contract

Each controller candidate SHALL contain a CAN ID, bounded nullable display-safe hardware/model name and firmware name, VESC version, and `vescSupported`. Candidates SHALL be sorted by CAN ID and MUST NOT be truncated when more than 16 motor controllers exist. `device.selectController` SHALL accept the active `deviceId` and a candidate `canId` and return the standard accepted result for a valid current supported selection.

#### Scenario: Current candidate selected

- **WHEN** `device.selectController` names the active device and one of its current candidates
- **THEN** the backend accepts exactly one selection and publishes newer authoritative state as negotiation resumes

#### Scenario: Selection in the wrong state

- **WHEN** a well-formed request targets a different device, an unsupported or non-candidate ID, a stale selection, or a connection not awaiting selection
- **THEN** the backend returns `invalid_state` and leaves state and persistence unchanged

#### Scenario: Malformed selection

- **WHEN** the request omits required fields or supplies an invalid CAN ID
- **THEN** the backend returns `invalid_request` and leaves state and persistence unchanged

#### Scenario: Selection merges current profile

- **WHEN** another Board Profile field changed after the selection candidates were published
- **THEN** a valid selection atomically merges only `preferredCanId` into the latest values and advances the profile revision once without overwriting that change

### Requirement: Optional preferred CAN ID

The versioned Board Profile SHALL accept `preferredCanId` as absent or null for automatic discovery or as an integer in the inclusive range `0` through `253`. The field SHALL be optional, clearable, revision-checked, and excluded from `missingFields` and profile completeness.

#### Scenario: Valid preference update

- **WHEN** `boardProfile.update` supplies a current revision and `preferredCanId` equal to `0`, `42`, `253`, or null
- **THEN** the backend persists the exact value at a newer revision and applies it on the next gateway connection, reconnect, or same-device retry

#### Scenario: Invalid preference update

- **WHEN** an update supplies a negative, fractional, nonnumeric, or greater-than-253 CAN ID
- **THEN** the backend returns `invalid_request` and leaves the Board Profile unchanged

#### Scenario: Selection persists preference

- **WHEN** `device.selectController` accepts a discovered CAN candidate
- **THEN** the saved board's Board Profile stores that ID and advances its revision without making the optional field required for completeness

### Requirement: Stable connection issue shape

`ActiveConnection.connectionIssue` SHALL be null unless unresolved asynchronous connection work has failed. Otherwise it SHALL contain one bounded message and exactly one of `transport_setup_failed`, `identification_timeout`, `no_controller_found`, `too_many_can_devices`, `too_many_controllers`, `preferred_controller_unavailable`, `controller_changed`, `unsupported_controller`, or `unsupported_refloat`. The failed active connection SHALL remain recoverable through retry or disconnect.

#### Scenario: Asynchronous configured-route failure

- **WHEN** an accepted connection later cannot validate its preferred CAN controller
- **THEN** authoritative state becomes `failed` with `preferred_controller_unavailable`, `unsupported_controller`, or `controller_changed` rather than relying on a past request response

#### Scenario: Retry starts

- **WHEN** the frontend retries the same failed device
- **THEN** the backend clears the prior issue before publishing the new connection generation

### Requirement: Detailed live telemetry contract

`LiveSample` SHALL expose one stable typed nullable shape for the detailed telemetry catalog across the mock, Apple, and Android backends. In addition to existing core values it SHALL represent motion and IMU values, rider inputs, requested and filtered control values, ATR and tilt contributions, package state and mode, stop and setpoint-adjustment state, wheelslip, darkride, charging, alert reason, field-weakening current, accumulated values, battery level, active firmware fault, one nullable ISO-8601 `slowObservedAt`, and a bounded extension map. The existing numeric `boardState` SHALL be superseded by `packageState`, and numeric `footpadState` SHALL become the stable footpad enum; numeric codes MAY remain only in bounded diagnostics. `LiveSample` SHALL NOT expose a derived torque or current-utilization value.

#### Scenario: Complete state snapshot

- **WHEN** detailed telemetry is available for the active connection
- **THEN** `state.get` and the next `state.changed` contain the same latest committed typed values, nulls for unavailable fields, and bounded extensions

#### Scenario: Cross-platform shape

- **WHEN** Apple and Android decode semantically equivalent responses
- **THEN** their authenticated JSON uses the same field names, enum values, units, nullability, and bounds

### Requirement: Stable detailed telemetry catalog

The typed catalog SHALL contain motion fields `speedKph`, `erpm`, `pitchDeg`, `balancePitchDeg`, and `rollDeg`; electrical fields `motorCurrentA`, `directionalMotorCurrentA`, `filteredMotorCurrentA`, `batteryCurrentA`, `requestedBalanceCurrentA`, `boosterCurrentA`, `fieldWeakeningCurrentA`, `dutyCycle`, `packVoltageV`, `controllerTempC`, and `motorTempC`; rider-input fields `footpadState`, `footpadAdc1V`, `footpadAdc2V`, and `remoteInput`; control fields `setpointDeg`, `atrSetpointDeg`, `brakeTiltSetpointDeg`, `torqueTiltSetpointDeg`, `turnTiltSetpointDeg`, `remoteSetpointDeg`, `atrAccelerationDifferenceErpmPerSecond`, and `atrSpeedBoost`; state fields `packageState`, `packageMode`, `stopCondition`, `setpointAdjustment`, `wheelslip`, `darkride`, `charging`, `alertReason`, and `firmwareFault`; and accumulated/charging fields `distanceM`, `odometerM`, `ampHoursDrawnAh`, `ampHoursChargedAh`, `wattHoursDrawnWh`, `wattHoursChargedWh`, `batteryLevel`, `chargingCurrentA`, and `chargingVoltageV`. `speedKph` SHALL use kilometres/hour; angular fields degrees; current fields amperes; voltage fields volts; temperatures degrees Celsius; distances metres; energy watt-hours; charge amp-hours; duty, battery level, ATR speed boost, and remote input decimal ratios; and ATR acceleration difference electrical RPM/second.

#### Scenario: Catalog serialization

- **WHEN** every catalog field is populated with a finite protocol-valid value
- **THEN** the mock, Apple, and Android serialize the exact stable names and units without aliases or platform-only fields

### Requirement: Stable detailed telemetry enums and units

Public detailed state values SHALL use bounded stable string enums with an `unknown` representation where forward compatibility requires it. `packageState` SHALL cover `disabled`, `startup`, `ready`, and `running`; `packageMode` SHALL cover `normal`, `handTest`, and `flywheel`; `footpadState` SHALL cover `none`, `left`, `right`, and `both`; `stopCondition` SHALL cover `none`, `pitch`, `roll`, `switchHalf`, `switchFull`, `reverseStop`, and `quickStop`; and `setpointAdjustment` SHALL cover `none`, `centering`, `reverseStop`, `speed`, `duty`, `error`, `highVoltage`, `lowVoltage`, and `temperature`. Alert reasons and firmware faults SHALL use the source-fixture allowlists plus `unknown`. Numeric values SHALL use the catalog units consistently in live samples, session series, diagnostics, and exports.

#### Scenario: Unknown future state

- **WHEN** a valid response contains an unrecognized package state, mode, stop condition, setpoint adjustment, or alert reason
- **THEN** the contract reports `unknown` for that semantic value without rejecting otherwise valid bounded telemetry

#### Scenario: Angular and electrical units

- **WHEN** pitch, setpoints, currents, voltage, energy, distance, and field-weakening data are serialized
- **THEN** every consumer observes the contract-defined degrees, amperes, volts, watt-hours, amp-hours, and metres without platform-specific conversion

### Requirement: Live and historical telemetry symmetry

Every typed detailed numeric field accepted into a durable session SHALL be retrievable through session-data series and samples under the same stable name and unit used by the live sample. Before pruning, exports SHALL include the same semantic fields and bounded extensions losslessly. After pruning, exports SHALL include every retained real source row plus unchanged events and summaries and SHALL state that removed samples cannot be reconstructed. `session.get` SHALL contain metadata only. Transition events SHALL be available through chronological range pages; each event SHALL contain committed sequence, source sample sequence, elapsed time, a kind from the detailed semantic categories, and a stable string value. Boolean transitions SHALL use `active` and `cleared`, nullable reason or fault clearing SHALL use `none`, and events SHALL be appended only when an observed discrete value changes. Series reduction MUST NOT average values, fabricate device history from before Float View observed the controller, or join across an unavailable or time-gap boundary.

#### Scenario: Review a simulated warning

- **WHEN** a duty-pushback sample is persisted and the session is later opened
- **THEN** historical series, exact samples, transitions, and export identify the same duty value, setpoint-adjustment state, and alert reason shown live

#### Scenario: Frontend recreation

- **WHEN** the WebView is recreated after detailed samples were committed
- **THEN** state, metadata, and authenticated session-data requests recover authoritative native values without depending on browser memory

#### Scenario: Review a discrete transition

- **WHEN** wheelslip becomes active and later clears during a session
- **THEN** chronological event pages contain one `wheelslip=active` event and one `wheelslip=cleared` event at their committed elapsed positions

#### Scenario: More than two thousand events

- **WHEN** a session contains transitions before and after its first 2,000 events
- **THEN** keyset pagination makes every committed transition reachable while no response contains an unbounded event collection

### Requirement: One controller per telemetry session

Live telemetry, transition events, durable samples, summaries, diagnostics, and exports SHALL belong to exactly one selected direct-or-CAN controller route at a time. Selecting or validating a different controller SHALL end the existing session and clear route-scoped telemetry before values from the new controller are accepted.

#### Scenario: Selected controller changes

- **WHEN** the active selection changes from a direct controller to a CAN-routed controller
- **THEN** the prior session ends before the first new-controller sample and no session metadata/data or export combines values from both controllers

### Requirement: Bounded telemetry serialization

Detailed live samples, extension keys and counts, metadata, session-list pages/cursors, session-data requests/results, retained sample pages, event pages, prune previews/jobs, and exports SHALL retain explicit bounds. Authenticated WebSocket messages SHALL remain at most 65,536 bytes. A negotiated field name SHALL be at most 64 UTF-8 bytes, an extension map SHALL contain at most 32 finite values, and serialized extension JSON SHALL be at most 16,384 UTF-8 bytes. A session-list page SHALL contain 1 through 100 summaries and an opaque cursor SHALL be at most 1,024 UTF-8 bytes. A session-data request body SHALL be at most 16,384 UTF-8 bytes; a decoded success body SHALL be at most 4,194,304 bytes; series and sample requests SHALL contain 1 through 16 unique fields; series SHALL contain 1 through 1,024 buckets and at most 65,536 total observations; sample pages SHALL contain at most 2,000 rows; and event pages SHALL contain at most 500 rows. Sequences, revisions, counts, and elapsed values SHALL be non-negative JavaScript-safe integers. Prune target resolution SHALL be a whole 1 through 300 seconds, and complete application state SHALL expose at most one bounded prune operation without embedding a ride-identifier collection. Launch tokens, platform transport identifiers, and full unrequested telemetry MUST NOT appear in state, history, extensions, errors, or default logs.

#### Scenario: Oversized extension data

- **WHEN** native telemetry contains extension data beyond the field-count, name, finite-value, or encoded-byte bound
- **THEN** the backend applies the specified deterministic rejection or omission rule and emits valid bounded contract state

#### Scenario: Composed series bound

- **WHEN** a requested field and bucket combination could exceed 65,536 observations or 4,194,304 decoded bytes
- **THEN** the backend rejects it before scanning or allocating a response even when each individual field and bucket value is otherwise valid

#### Scenario: Privacy inspection

- **WHEN** live state, metadata, session-data responses, exports, errors, and logs are inspected
- **THEN** canonical learned or simulated VESC UUIDs may be present but no launch token or platform transport identifier is present

### Requirement: Bounded pageable standalone session catalog

The WebSocket control contract SHALL expose `session.list` for board-scoped session summaries outside complete `state.changed` snapshots. The request SHALL require a saved-board identifier, accept a page size from 1 through 100 and an optional opaque cursor, and order by `startedAt` descending and `sessionId` descending as a deterministic tie-break. The result SHALL contain at most the requested number of summaries, the catalog revision observed by the query, and either a bounded continuation cursor or null. Cursors SHALL be versioned and bound to the required board filter, query shape, catalog revision, and last `(startedAt, sessionId)` key; a cursor for another board, query, revision, or malformed context SHALL fail with bounded conflict or invalid-request semantics. The list SHALL contain no grouping metadata or linked-session state. An unfiltered/global session-list query and its backend callers SHALL not remain. Guest connections SHALL never create rows returned by this operation.

#### Scenario: First board-filtered page

- **WHEN** the frontend requests a valid page with a saved-device identifier
- **THEN** the backend returns only that board's newest standalone session summaries in deterministic order and no more than the requested count

#### Scenario: Missing board filter

- **WHEN** the frontend requests `session.list` without a saved-device identifier
- **THEN** the backend returns `invalid_request` and does not execute an unfiltered catalog query

#### Scenario: Continue beyond the page bound

- **WHEN** more sessions exist for the selected board after the first page
- **THEN** the result supplies a cursor and later pages reach every remaining summary for that board without offset scanning or duplication

#### Scenario: Cursor filter conflict

- **WHEN** a board-filtered cursor is reused with another board filter, query shape, or catalog revision
- **THEN** the backend rejects it with the same bounded conflict or invalid-request semantics and returns no mixed page

#### Scenario: Guest isolation

- **WHEN** a guest connection starts and ends after producing live samples
- **THEN** `session.list` returns no guest session and the catalog revision is unchanged by those samples

### Requirement: Bounded compressed session-data protocol

The hand-written contract SHALL define explicit columnar JSON request/result unions for `series`, `samples`, and `events`. Every request SHALL identify one session, one previously published snapshot including history revision, one bounded inclusive elapsed range, and only the fields, gap policy, bucket/page limits, mode, and cursor applicable to its kind. Successful results SHALL echo the session, snapshot, revision, range, and applicable gap policy; SHALL use parallel arrays with validated equal lengths; SHALL advertise standard HTTP gzip content encoding when accepted; and SHALL remain inside the composed observation, collection, and decoded-byte bounds. The frontend SHALL count the decoded response stream before parsing and SHALL reject malformed, mismatched, excessive, unsupported, or unrequested data.

#### Scenario: Compressed range query

- **WHEN** an authenticated client requests valid fields and an elapsed range with gzip accepted
- **THEN** the backend returns gzip-encoded columnar JSON whose decoded result satisfies every declared field, array, observation, and byte bound

#### Scenario: Oversized decoded stream

- **WHEN** a response stream expands beyond 4,194,304 decoded bytes or declares inconsistent parallel arrays
- **THEN** the frontend aborts and reports a bounded invalid-backend response without parsing or retaining the complete body

#### Scenario: Client cancellation

- **WHEN** the frontend aborts a superseded overview, zoom, sample, or event request
- **THEN** the backend requests native interruption, eventually releases all work and leases, and remains responsive to durable persistence and a later valid request without relying on a correctness timeout

### Requirement: Resolution-aware historical series

A `series` query SHALL select one published sample boundary and history revision, inclusive elapsed range, ordered unique field list, target bucket count, and the nullable persisted history-gap threshold captured by the frontend. For each field the backend SHALL read samples in `(elapsedMs, sequence)` order and select first, minimum, maximum, and last non-null observations per non-empty bucket, deduplicating observations that occupy multiple roles and returning them in source order. Bucket arithmetic SHALL assign each endpoint deterministically without overflow. A null or absent raw observation SHALL advance that field's continuity generation. An elapsed delta greater than a configured threshold SHALL advance every field; when unset, the edge threshold SHALL be `max(1 second, 3 × session-captured expected persistence interval for the preceding ride phase, 3 × retained history resolution)`, with zero retained-resolution contribution for unpruned history. The backend SHALL read phase internally without returning it unless requested. Returned segments SHALL join selected observations only within one unchanged generation. The result SHALL preserve finite extrema and endpoints, contain at most four observations per bucket per field, and omit every unrequested telemetry value.

#### Scenario: Four-hour overview

- **WHEN** a 150,000-sample session is queried across its full elapsed range for a 400-pixel chart
- **THEN** each requested field returns at most 1,600 ordered observations regardless of raw sample count

#### Scenario: Narrow spike and voltage sag

- **WHEN** a bucket contains a short positive spike and a short lower value between its first and last observations
- **THEN** both extrema retain their original elapsed time and sequence position rather than being replaced by an average

#### Scenario: Alternating missing observations

- **WHEN** valid and null values alternate repeatedly within one bucket
- **THEN** the response remains within four selected observations and no returned segment instructs the chart to bridge a raw null run

#### Scenario: Configured recording gap

- **WHEN** adjacent non-null rows are farther apart than the configured threshold in seconds
- **THEN** every requested field starts a new continuity segment at the later row

#### Scenario: Derived recording gap

- **WHEN** no threshold is configured and a gap is compared for a lossless or pruned ride phase
- **THEN** the backend uses the maximum of one second, three expected preceding-phase persistence intervals, and three retained-resolution intervals

#### Scenario: Field minimization

- **WHEN** a query requests two fields from a session containing the complete stable catalog and extensions
- **THEN** storage selection and the response contain only those two fields plus ordering/snapshot columns

### Requirement: Exact samples and pageable transition events

A `samples` window query SHALL return chronological exact retained source rows for requested fields in keyset pages of at most 2,000 with explicit continuation state. An `events` query SHALL return chronological committed transitions for an elapsed range in keyset pages of at most 500. Sample and event cursors SHALL be opaque, bounded, versioned, and bound to their session, history revision, snapshot, range, and last `(elapsedMs, sequence)` key so all results remain reachable without duplication or cross-request replay.

#### Scenario: Dense diagnostic window

- **WHEN** a valid retained-source range contains more than 2,000 committed rows
- **THEN** the first page reports continuation and subsequent cursors reach the remaining rows without silently truncating or loading the complete range

#### Scenario: Invalid cursor replay

- **WHEN** a sample or event cursor is reused with another session, snapshot, or elapsed range
- **THEN** the backend rejects it with bounded `invalid_request` semantics before returning data

### Requirement: Irreversible bounded ride pruning

The backend SHALL support cancellable preview and execution of pruning for one completed ride or all rides completed when a batch starts. Only one preview or mutation SHALL run at once. A preview SHALL share bounded historical reader concurrency, process batch rides sequentially, and return bounded aggregate counts rather than a ride list. A prune SHALL use elapsed-zero-aligned whole-second buckets and retain only real source rows: session endpoints, the first non-null source for each recorded numeric field, both sides of observed riding-phase boundaries, event source samples, deterministic summary-maximum source samples, and at most eight additional rows per bucket selected as bucket endpoints then by deterministic maximum coverage of ordered per-field minimum/maximum roles. Equal summary maxima SHALL protect the earliest `(elapsedMs, sequence)` source. Selection SHALL stream with memory bounded by field count and one bucket and SHALL use bounded-lifecycle file-backed SQLite staging proportional to retained sequences rather than raw history. It SHALL never prune an active or deleting ride. Re-pruning SHALL require a strictly coarser integer-multiple resolution. Each ride SHALL commit or roll back atomically, preserve recorded count, available-field metadata, summaries, transitions, and elapsed coverage, update stored count/policy, and increment history revision. Batch completion SHALL be per-ride rather than all-or-nothing, SHALL be bounded by a captured upper session key, and SHALL exclude rides completed after the batch started. Cancellation observed after a ride commits SHALL reconcile and count that ride before stopping.

#### Scenario: Preview one ride

- **WHEN** a valid completed ride and 1-through-300-second target resolution are previewed
- **THEN** the backend returns a cancellable read-only estimate of retained rows and approximate reclaimable bytes without changing history revision or storage

#### Scenario: Prune one ride

- **WHEN** pruning a completed ride commits successfully
- **THEN** only deterministically retained source rows remain, protected evidence and original summaries/events remain reachable, stored count decreases or remains equal, and all older snapshots conflict with the newer history revision

#### Scenario: Batch cancellation

- **WHEN** the user cancels an all-rides prune after some rides commit and while another ride is being processed
- **THEN** completed rides remain pruned, the current ride rolls back unless already committed, later rides remain unchanged, and bounded application state reports the terminal progress

#### Scenario: Invalid re-prune

- **WHEN** a previously pruned ride is requested at an equal, finer, non-multiple, or misaligned resolution
- **THEN** a single-ride request is rejected and a batch request reports that ride as skipped without changing its revision

#### Scenario: Pruned export

- **WHEN** a pruned ride is exported or exact samples are queried
- **THEN** only retained exact source rows are returned and pruning policy plus recorded-versus-stored counts disclose that the original lossless stream is unavailable

### Requirement: Stable published session snapshots

The per-session database SHALL durably identify the history revision, latest committed sample sequence, event sequence, recorded/stored sample counts, event count, and elapsed boundary. `session.get` SHALL expose only a boundary that has also converged into the catalog and authoritative application state. Every session-data response SHALL echo the requested revision and boundary. Older published active-session boundaries SHALL remain queryable after newer commits within the same revision, later rows SHALL be excluded by sequence, and a future, stale-revision, or impossible boundary SHALL fail without mixing snapshots. Native persistence SHALL remain authoritative while the frontend is absent.

#### Scenario: Query while persistence continues

- **WHEN** an active session commits additional samples while the frontend queries an earlier published boundary
- **THEN** every result is internally consistent through that earlier boundary and logging continues without waiting for the historical scan

#### Scenario: Crash between databases

- **WHEN** the session database commits a newer marker but the process stops before the catalog update
- **THEN** startup reconciles the catalog from that marker before publishing recovered metadata

#### Scenario: Refresh growing history

- **WHEN** a later metadata request reports a newer published boundary
- **THEN** the same data protocol can query the expanded active history without changing request or rendering semantics

#### Scenario: Refresh pruned history

- **WHEN** metadata reports a newer history revision after pruning
- **THEN** prior cursors and snapshots conflict and refreshed requests observe one internally consistent retained source history

### Requirement: Location setting and permission state

The authenticated contract SHALL add `locationRecordingEnabled` and `includeGpsLocationWhenExporting` to the complete persisted settings shape and SHALL expose bounded location authorization/status and action availability through authoritative state. Both settings SHALL default to `false`, and the state shape SHALL not contain location coordinates.

#### Scenario: Settings round trip

- **WHEN** the frontend updates `locationRecordingEnabled` at the current settings revision
- **THEN** the backend persists and returns the boolean at a newer revision on every supported target

#### Scenario: State status

- **WHEN** location authorization or recording eligibility changes
- **THEN** the backend publishes a newer complete state containing bounded status/action information without a location track

### Requirement: Location permission actions

The mandatory contract SHALL provide platform-neutral requests to ask for location permission and to open platform settings when the OS requires settings navigation. These methods SHALL use the existing stable error envelopes and SHALL be safe to invoke repeatedly.

#### Scenario: Request permission

- **WHEN** the frontend invokes the location permission action
- **THEN** the backend requests the next supported OS permission and returns the resulting bounded status or `permission_required`/`unavailable` error

#### Scenario: Open settings

- **WHEN** the user invokes the settings action after a non-repromptable denial
- **THEN** the native host presents the platform settings destination or returns a bounded `unavailable` error without altering board connection state

### Requirement: Published location boundary

Session metadata and the published session snapshot SHALL include bounded location availability/count and the last durable location sequence/elapsed boundary. A successful location append SHALL advance the published boundary only after the corresponding SQLite write is durable.

#### Scenario: Durable append

- **WHEN** a native location batch commits successfully
- **THEN** the next session metadata and state revision expose the new location boundary

#### Scenario: Partial write

- **WHEN** a location batch fails before commit
- **THEN** the published boundary remains at the previous durable row and a bounded internal error is reported without coordinates

### Requirement: Authenticated locations session-data method

The session-data contract SHALL add a bounded `locations` kind with cursor, limit, time-range, and published-snapshot validation semantics matching existing session-data kinds. Results SHALL return location points and explicit gaps without embedding them in `AppState` or `session.get` history payloads.

#### Scenario: Valid query

- **WHEN** the frontend sends a correctly authenticated bounded `locations` request for the current published snapshot
- **THEN** the backend returns one bounded page with a continuation cursor when applicable

#### Scenario: Invalid query

- **WHEN** the request exceeds the documented limit, names an unknown session, or uses a stale snapshot
- **THEN** the backend returns a stable bounded error and allocates no unbounded result

### Requirement: Irreversible saved-board deletion

The `savedDevice.delete` method SHALL accept a saved-device identifier and return `Accepted` only after the native owner has removed that saved board, its Board Profile, private routing metadata, and all session files and catalog rows owned by its `deviceHistoryId`. The operation SHALL be irreversible, SHALL increment catalog state once after cleanup, and SHALL fail with `invalid_state` when that board has an active connection. It SHALL be idempotently rejected as `not_found` after the board has already been removed.

#### Scenario: Delete inactive board

- **WHEN** the frontend requests `savedDevice.delete` for an inactive saved board
- **THEN** the backend removes the board and all of its owned sessions in one native operation and returns `{ accepted: true }`

#### Scenario: Active board deletion is rejected

- **WHEN** `savedDevice.delete` races with or follows an active connection for the same board
- **THEN** the backend returns `invalid_state` and leaves the board, profile, routing metadata, and sessions unchanged

#### Scenario: Delete updates authoritative state

- **WHEN** an inactive board deletion completes
- **THEN** the next complete state and board-scoped session requests omit the deleted board and its sessions, while other boards remain available

### Requirement: Portable session archive board ownership

The existing native session archive export/import flow SHALL carry the canonical VESC UUID as `deviceHistoryId` when meaningful, canonical controller/BMS UUID metadata where available, and the bounded saved-board snapshot needed to reconstruct board ownership. It MUST NOT carry Apple CoreBluetooth UUIDs or Android Bluetooth addresses. Import SHALL merge into the saved board with the same `deviceHistoryId`, or create that board before inserting sessions when no match exists; the resulting board SHALL have a nullable local platform association.

#### Scenario: Cross-platform archive merge

- **WHEN** an archive is imported on another supported platform and its `deviceHistoryId` already exists
- **THEN** the sessions are attached to the existing board without requiring the source platform identifier or creating a duplicate board

#### Scenario: Cross-platform archive creates board

- **WHEN** an archive is imported and its `deviceHistoryId` does not exist locally
- **THEN** the backend creates a configurable saved-board entry with no local platform association and attaches all imported sessions to it

#### Scenario: Archive without board identity

- **WHEN** an archive has no valid stable `deviceHistoryId`
- **THEN** import fails with the bounded invalid-archive error and does not guess a board or partially insert sessions

#### Scenario: Portable identity privacy

- **WHEN** a session archive is written or read
- **THEN** the stable opaque board identity is available for association but raw hardware and platform identifiers are absent from the portable data

### Requirement: Explicit connection tracking intent

The `device.connect` method SHALL accept `{ deviceId, trackingMode }`, where `trackingMode` is exactly `saved` or `guest`. The backend SHALL reject a missing or unknown mode with `invalid_request`, retain the accepted mode on the connection generation, and use it for retries, reconnect, WebView recreation, session lifecycle, location capture, controller preference writes, and learned routing hints. The frontend SHALL send `saved` only for a known local saved-board association or an explicit Save and connect choice, and SHALL send `guest` only for an explicit Connect as guest choice.

#### Scenario: Saved intent

- **WHEN** Home sends `device.connect` with `trackingMode: saved`
- **THEN** the backend follows the saved lifecycle and may establish or resolve saved-board identity only after valid VESC identification

#### Scenario: Guest intent

- **WHEN** Home sends `device.connect` with `trackingMode: guest`
- **THEN** the backend follows the guest lifecycle and never promotes the connection to saved persistence

#### Scenario: Invalid intent

- **WHEN** `device.connect` omits or changes `trackingMode` to an unsupported value
- **THEN** the backend returns `invalid_request` before starting a connection generation

### Requirement: Authoritative active device kind

Every non-null active connection snapshot SHALL contain required `deviceKind` equal to `null` while the direct endpoint is not yet classified, `board` when a motor-controller route owns the board experience, or `bms` when an exact directly connected supported VBMS owns a BMS-only experience. The value MUST be generation-scoped and MUST NOT be inferred by the frontend from display names, platform identifiers, firmware labels, nullable controller fields, or BMS status.

#### Scenario: Identification is pending

- **WHEN** a connection exists before direct firmware classification completes
- **THEN** its complete authoritative snapshot contains `deviceKind: null`

#### Scenario: Direct VBMS is classified

- **WHEN** the direct firmware identity matches the exact supported VBMS catalog
- **THEN** the same generation publishes `deviceKind: bms` before or with BMS-only readiness

#### Scenario: Controller route is established

- **WHEN** a direct or CAN-routed motor controller becomes the selected board controller
- **THEN** the active connection publishes `deviceKind: board`

#### Scenario: Stale classification arrives

- **WHEN** firmware or polling work from an old generation completes after disconnect or reconnect
- **THEN** it cannot change the current active connection's device kind

### Requirement: Complete live BMS contract state

`ActiveConnection` SHALL contain one mandatory bounded `bms` value with status `discovering`, `not_found`, `attached`, `unavailable`, or `ambiguous`; nullable direct-or-CAN connection; at most 16 display-safe candidates; and nullable complete latest snapshot. The snapshot SHALL contain fixed platform-neutral fields for observation time, voltages, currents, session counters, cell voltages and balancing, temperatures, humidity, state of charge and health, reported CAN ID, lifetime counters, pressure, data version, and status.

#### Scenario: Attached BMS

- **WHEN** the backend receives and completely decodes values directly from the selected VBMS
- **THEN** a newer authoritative state reports `attached`, the verified route, no ambiguity candidates, and one complete snapshot with bounded arrays

#### Scenario: Ambiguous BMS

- **WHEN** multiple unconfigured VBMS identities are detected
- **THEN** authoritative state reports `ambiguous`, a null selected connection and snapshot, and CAN-ID-sorted bounded candidates with canonical UUIDs when meaningful

#### Scenario: Unavailable after a prior sample

- **WHEN** a previously attached VBMS exceeds its consecutive failure threshold
- **THEN** authoritative state reports `unavailable` while retaining the last snapshot and its original observation time

#### Scenario: Frontend recreation

- **WHEN** a disposable frontend reconnects during BMS discovery, ambiguity, attachment, unavailability, or controller reconnect
- **THEN** `state.get` supplies the complete current BMS value without another platform-specific request

### Requirement: Live-only BMS event semantics

Every changed BMS status, route, candidate set, or successful complete BMS snapshot SHALL advance the application-state revision and publish the newest conflated `state.changed` snapshot. BMS snapshots MUST NOT be persisted as session samples or diagnostics and MUST NOT change session counts, aggregates, integrity, review, or exports.

#### Scenario: Successful BMS poll

- **WHEN** a complete BMS response differs by observation time or value from authoritative state
- **THEN** the backend publishes it through a newer complete application snapshot without appending a session sample

#### Scenario: Slow frontend

- **WHEN** BMS polls produce state faster than the frontend consumes events
- **THEN** pending delivery converges on the newest complete bounded BMS snapshot without retaining an unbounded history

#### Scenario: Session export

- **WHEN** a session recorded while live BMS values were visible is reviewed, diagnosed, or exported
- **THEN** its persisted shapes and results contain no BMS snapshot arrays or BMS-only counters introduced by this change

### Requirement: Preferred BMS CAN profile field

Every Board Profile SHALL accept nullable or absent `preferredBmsCanId` for automatic BMS discovery or an integer from 0 through 253 for one authoritative CAN route. The field SHALL be revision checked, SHALL NOT contribute to profile completeness, and SHALL remain independent from the controller's `preferredCanId`.

#### Scenario: Valid BMS preference

- **WHEN** the frontend updates `preferredBmsCanId` to `42` at the current Board Profile revision
- **THEN** both backends persist exactly `42`, advance the profile revision once, preserve unrelated fields, and apply it to the current auxiliary generation and subsequent connections

#### Scenario: Cleared BMS preference

- **WHEN** the frontend clears `preferredBmsCanId`
- **THEN** the profile returns to bounded automatic BMS discovery without clearing or changing `preferredCanId`

#### Scenario: Invalid BMS preference

- **WHEN** a patch supplies a negative, fractional, nonnumeric, or greater-than-253 `preferredBmsCanId`
- **THEN** every backend rejects it with the standard invalid-request semantics and leaves persisted profile and active BMS routing unchanged

#### Scenario: Active BMS-only reconfiguration

- **WHEN** a valid `preferredBmsCanId` update targets the active saved board
- **THEN** controller route, connection readiness, and active session remain unchanged while a newer auxiliary generation resolves the requested BMS route

### Requirement: Bounded Refloat control contract

`ActiveConnection.refloatControls` SHALL be null before Refloat discovery begins and otherwise SHALL contain `status` as `loading`, `ready`, or `unavailable`; `writable`; nullable `packageEnabled` and `flywheelEnabled`; `lightingAvailable`; nullable `ledsEnabled` and `headlightsEnabled`; `fatalErrorClearAvailable`; nullable `fatalErrorLatched`, `fatalErrorActive`, and `fatalErrorClearable`; and nullable `pendingAction` from `package`, `flywheel`, `leds`, `headlights`, or `clearFatalError`. `refloat.control.set` SHALL accept exactly one `control` from `package`, `flywheel`, `leds`, or `headlights` and one boolean `enabled`. `refloat.fatalError.clear` SHALL accept empty parameters. A successful method SHALL return the complete confirmed `RefloatControls`; unsafe, read-only, stale, unavailable, or competing requests SHALL use the existing stable error vocabulary and MUST NOT return optimistic partial state.

#### Scenario: Idempotent requested value

- **WHEN** `refloat.control.set` requests a value already confirmed by current authoritative state
- **THEN** the backend returns the unchanged complete controls without sending a board mutation

#### Scenario: Confirmed control result

- **WHEN** an accepted control is confirmed by its protocol response or later telemetry
- **THEN** the request returns complete controls matching the newer `state.changed` snapshot and the pending action is null

#### Scenario: Invalid control name

- **WHEN** `refloat.control.set` supplies an unknown control or malformed parameter
- **THEN** the backend returns `invalid_request` without sending a board command

#### Scenario: Control unavailable

- **WHEN** a control is requested before controls are ready, during reconnect, for absent lighting or fatal capability, on read-only firmware, or while flywheel mode prohibits that mutation
- **THEN** the backend returns `invalid_state` without changing board or authoritative control state

#### Scenario: Flywheel disable remains actionable

- **WHEN** authoritative state reports active flywheel mode and the user requests flywheel disabled, including while its package run state is normalized as riding
- **THEN** the backend may perform only the exact source-defined flywheel-disable operation while every other mutating control remains unavailable

#### Scenario: Confirmation timeout

- **WHEN** an accepted action cannot obtain its required bounded response or confirmation
- **THEN** the backend returns `unavailable`, clears `pendingAction`, and retains the last confirmed values

### Requirement: Bounded controller shutdown contract

`ActiveConnection.controllerActions` SHALL contain boolean `shutdownAvailable` and nullable `pendingAction`, whose only non-null value is `shutdown`. `controller.shutdown` SHALL accept empty parameters and return the existing `Accepted` result after the exact one-way request is accepted for transmission. `shutdownAvailable` SHALL be true only while the current selected route is ready, writable, on an audited stable VESC shutdown profile, authoritatively idle, outside flywheel mode, and free of another pending mutation. The result and state MUST NOT claim that controller power-off was confirmed.

#### Scenario: Shutdown accepted for transmission

- **WHEN** an eligible idle connection requests `controller.shutdown`
- **THEN** the backend publishes shutdown-pending state, submits exactly one non-forced non-restart request, and returns `Accepted` without fabricating a device acknowledgement

#### Scenario: Unsupported controller remains connected

- **WHEN** the accepted shutdown request is ignored by controller hardware and the selected route remains available through the bounded intent window
- **THEN** the backend clears shutdown-pending state, continues the active connection, and does not report confirmed power-off

#### Scenario: Expected loss after shutdown

- **WHEN** the selected controller or BLE transport disappears while current shutdown intent is pending
- **THEN** the backend ends the active connection as user initiated and suppresses automatic reconnection

#### Scenario: Shutdown unavailable

- **WHEN** shutdown is requested on VESC 6.02, an unaudited or read-only firmware build, a non-ready or stale connection, a non-idle board, flywheel mode, or while another mutation is pending
- **THEN** the backend returns `invalid_state`, sends no command, and preserves authoritative state apart from any bounded error

### Requirement: Bounded Refloat configuration contract

Every supported backend SHALL implement mandatory `refloat.configuration.open`, `refloat.configuration.schema`, `refloat.configuration.read`, and `refloat.configuration.write` methods plus `refloat.tune.list`, `refloat.tune.get`, `refloat.tune.create`, `refloat.tune.update`, `refloat.tune.delete`, and `refloat.tune.apply`. Open SHALL return the small active identity/status summary; schema SHALL return one cursor-bounded group/subgroup page; read SHALL accept exactly `current` or `default`; write SHALL accept exact package version, schema signature, base current revision, and a complete bounded typed value map. Tune mutation methods SHALL use opaque IDs and revisions. All results, fields, strings, maps, pages, cursors, and errors MUST fit documented bounds and use the stable error vocabulary.

#### Scenario: Supported backend

- **WHEN** the shared frontend uses Refloat configuration on mock, Android, iOS, or macOS
- **THEN** every mandatory method has identical request, result, bounds, revision, and error semantics

#### Scenario: Paged schema

- **WHEN** a normalized schema and its descriptions would exceed one control message
- **THEN** the frontend retrieves deterministic bounded pages using opaque cursors without receiving raw XML or compressed bytes

#### Scenario: Stale write

- **WHEN** write package identity, signature, connection generation, or base current revision is stale
- **THEN** the backend returns `conflict` or `invalid_state` without sending a board command

#### Scenario: Oversized values or metadata

- **WHEN** any configuration or Tune request/result exceeds its field, string, collection, cursor, or encoded-message limit
- **THEN** the backend rejects it before unbounded parsing, allocation, persistence, or Bluetooth work

### Requirement: Small authoritative configuration availability

`ActiveConnection.refloatConfiguration` SHALL be null before Refloat discovery begins and otherwise contain only bounded status from `loading`, `ready`, or `unavailable`; writability; nullable exact package version until legacy schema identity resolves; nullable schema fingerprint and wire signature; current/default-loaded flags; and nullable pending operation from `schema`, `readCurrent`, `readDefault`, `write`, or `applyTune`. It MUST NOT contain schema fields, raw XML, raw values, saved configurations, or cache keys. Entry into flywheel mode SHALL clear the current-loaded flag and writability without discarding validated schema/default availability; confirmed exit SHALL not restore them until a fresh current read succeeds. Availability and pending-operation changes SHALL publish newer complete authoritative snapshots, while bulk schema/value results remain request-scoped.

#### Scenario: Ready configuration

- **WHEN** schema validation and a fresh current read complete for a ready writable connection
- **THEN** the next authoritative snapshot reports ready and writable with matching version/signature and loaded current values

#### Scenario: Reconnecting connection

- **WHEN** the active device reconnects
- **THEN** configuration becomes loading and non-writable while retained bulk request results cannot authorize a write

#### Scenario: Slow frontend

- **WHEN** telemetry snapshots continue while a schema page or value request is outstanding
- **THEN** authoritative state remains bounded and conflated without embedding or repeatedly delivering the bulk result

#### Scenario: Flywheel configuration contract

- **WHEN** authoritative package mode is flywheel
- **THEN** `read(current)`, `write`, and `tune.apply` return `invalid_state`, schema/default reads may remain available, and the summary reports no loaded authoritative current values
