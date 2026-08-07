## ADDED Requirements

### Requirement: One shared native backend

iOS, macOS, and Android SHALL use the portable Rust backend under `/backends/core` for application state, VESC and Refloat protocols, persistence, loopback transport, contract dispatch, exports, and backend-owned jobs. A newly supported native host SHALL integrate that same core through the versioned platform bridge rather than copying it.

#### Scenario: Core behavior change

- **WHEN** shared backend behavior changes
- **THEN** every shipped host receives the same Rust implementation and contract semantics without a corresponding translation

#### Scenario: Future host starts

- **WHEN** a Linux or Windows host is proposed later
- **THEN** its backend work is limited to host wiring, bridge integration, and required platform adapters plus gaps explicitly approved as core changes

### Requirement: Thin Android host

The Android application SHALL retain Kotlin only for application entry, Activity/WebView and lifecycle ownership, Android Bluetooth/location/background-service adapters, bounded `AssetManager` resource lookup, content-URI document selection/staging, privacy-safe diagnostics, permission/settings/theme/export presentation, the dedicated host-command worker, and the small JNI wrapper. Kotlin MUST NOT retain archive validation/import semantics, state publication, frontend event dispatch, a second application service, protocol stack, SQLite store, loopback router, frontend contract implementation, or native callback object graph.

#### Scenario: Android backend operation

- **WHEN** the Android frontend invokes a backend method
- **THEN** the Rust loopback server validates and executes it while Kotlin handles only platform commands or lifecycle inputs selected through the bounded JNI bridge

#### Scenario: Android document picker workaround

- **WHEN** an Android WebView requires native content-URI handling for a selected import
- **THEN** Kotlin stages the selected bytes at the core-provided bounded destination and reports completion while Rust validates/imports the archive and publishes every backend/frontend result

#### Scenario: Android Activity recreation

- **WHEN** Android replaces its Activity or WebView without killing the process
- **THEN** Kotlin replaces presentation wiring and its bridge worker without creating a second Rust runtime or recovering state through Kotlin

#### Scenario: Android process recreation

- **WHEN** the operating system destroys and later recreates the Android process
- **THEN** the application creates exactly one new Rust runtime, invalidates prior native handles and presentations, and restores only durable core state

## MODIFIED Requirements

### Requirement: Lowercase ownership layout

The repository SHALL place the frontend under `/frontend`, the portable Rust workspace under `/backends/core`, Apple hosts and adapters under `/backends/apple`, the Android host and adapters under `/backends/android`, shared backend SQL and immutable protocol fixtures under `/backends`, canonical acceptance under top-level `/conformance`, and supporting firmware under `/tools/vesc-test-device`.

#### Scenario: Repository navigation

- **WHEN** a developer inspects the repository root
- **THEN** lowercase ownership paths distinguish the portable backend from thin platform hosts, shared immutable resources, conformance, tools, and planning

### Requirement: Thin Apple hosts

The iOS and macOS hosts SHALL contain only application entry, lifecycle, WebView, Apple Bluetooth/location adapters, bounded bundle-resource lookup, security-scoped document selection/staging, privacy-safe diagnostics, permission/settings/theme/export presentation, a small hand-written Swift C-ABI wrapper, one dedicated command-pump worker, and platform resource wiring not owned by the Rust backend. Both hosts SHALL consume the same generated C header and Rust library build identity. Apple hosts MUST NOT become the behavioral authority for shared code; contract/specification/conformance evidence governs rewrite discrepancies.

#### Scenario: WebView replacement

- **WHEN** an Apple host replaces its WebView
- **THEN** it reconnects to the same process-owned Rust backend state without reimplementing backend behavior or creating another runtime

#### Scenario: Core dependency update

- **WHEN** a Rust or C-ABI dependency, tool, or generated header changes
- **THEN** iOS, macOS, Android, and core tests consume the same locked source identity and compatible ABI build identity

#### Scenario: Rewrite discrepancy

- **WHEN** ported Rust behavior disagrees with the hand-written contract, requirement, immutable fixture, canonical SQL, or canonical conformance result
- **THEN** the Rust core is corrected to the authoritative evidence rather than preserving Swift or Kotlin behavior by default

### Requirement: One Android application module

Android under `/backends/android` SHALL use one Gradle application module. Host boundaries SHALL be represented by focused Kotlin adapter files and one small JNI wrapper, while one Gradle-owned locked Cargo cross-compilation invocation builds the Rust backend exactly once for each packaged ABI.

#### Scenario: Android build

- **WHEN** the Android application is assembled
- **THEN** the one application module builds or consumes the matching locked Rust artifacts, packages each required ABI library exactly once, validates the JNI/ABI identity before packaging, and contains no second Kotlin backend implementation after cutover

### Requirement: Mirrored native connection ownership

The Rust backend SHALL be the single owner for application coordination, controller discovery, telemetry polling, Bluetooth stream policy, VESC framing and commands, firmware parsing, exact compatibility mapping, and Refloat protocol families. Platform hosts SHALL implement only bounded Bluetooth and lifecycle adapter commands/events. Dependencies MUST point downward from application coordination to port models and protocol components, and superseded Swift/Kotlin mirrored paths MUST be deleted after their host cutover gates pass.

#### Scenario: Locate controller discovery

- **WHEN** a developer needs to change direct-or-CAN route selection
- **THEN** one portable Rust controller-discovery owner is available rather than matching host implementations

#### Scenario: Locate a protocol family

- **WHEN** a developer needs to change VESC framing or a Refloat wire layout
- **THEN** one focused Rust codec family is found under the core protocol ownership tree

#### Scenario: Add a compatible firmware version

- **WHEN** a later VESC or Refloat release uses an existing tested wire family
- **THEN** the Rust backend adds one compatibility-catalog entry and fixtures without copying application-service, routing, or polling code

#### Scenario: Add a new wire family

- **WHEN** a later release changes a required wire layout
- **THEN** the Rust backend changes the focused codec family and maps exact versions to it while framing, routing, the native ABI, and every host remain unchanged

#### Scenario: Backend source documentation

- **WHEN** a developer reads backend ownership documentation
- **THEN** it identifies Rust crate locations, platform adapter locations, dependency direction, privacy boundary, C/JNI interoperability boundary, and version-to-family extension rule

### Requirement: Mirrored detailed telemetry ownership

The Rust backend SHALL own Refloat family decoding, telemetry scheduling, semantic normalization, application state, durable persistence, session retrieval, and serialization once for every host. VESC command routing SHALL remain below Refloat polling, and neither protocol owner SHALL depend on Bluetooth implementations, storage, loopback transport, host interoperability, or frontend code.

#### Scenario: Add a known telemetry field

- **WHEN** support for an existing negotiated Refloat field is added
- **THEN** one Rust decoder, normalized model, persistence mapping, serializer, fixture set, and test set is updated without host-specific frontend or backend logic

#### Scenario: Route ownership

- **WHEN** a detailed Refloat request targets a CAN controller
- **THEN** the Refloat owner supplies the command and the Rust VESC routing owner applies CAN forwarding exactly once

### Requirement: Mirrored native session-data adapters

The Rust backend SHALL implement every authenticated session-data request kind over its bundled SQLite session store. It SHALL validate the shared hand-written contract, acquire the requested published history revision and boundary, select only validated columns/ranges plus internal ride phase, stream ordered rows through the bounded configurable-gap first/minimum/maximum/last reducer, serialize the explicit columnar response, and apply standard gzip encoding. Shared behavior SHALL be enforced through Rust tests, canonical SQL, immutable vectors, mock, and conformance rather than host-specific runtime adapters or a generated contract.

#### Scenario: Equivalent historical query

- **WHEN** different hosts query equivalent stored rows with the same series, samples, or events request
- **THEN** the shared Rust implementation returns semantically identical snapshots, columnar values, extrema, continuity segments, ordering, and continuation behavior

#### Scenario: Portable compression

- **WHEN** the frontend accepts gzip from any host
- **THEN** the Rust backend returns the same decoded contract using its one bounded compression owner

#### Scenario: Backend ownership

- **WHEN** the WebView is destroyed, recreated, or aborts a query
- **THEN** Rust storage, active logging, snapshot publication, cancellation, recovery, and cleanup remain owned by the process runtime

### Requirement: Mirrored saved and guest tracking policy

The Rust backend SHALL implement one explicit tracking-mode policy for every host. The policy SHALL be selected before connection work begins, retained across reconnect and WebView recreation, and checked at every durable side-effect boundary including saved-device creation, Board Profile writes, controller preference or learned-hint writes, session creation/append/deletion, location recording, catalog revision changes, and exportable ride metadata. Saved mode SHALL use durable owners; guest mode SHALL use bounded in-memory live state and perform none of those durable writes. Platform hosts MUST NOT implement or override this policy.

#### Scenario: Saved native connection

- **WHEN** any native host receives saved connection intent
- **THEN** the Rust backend creates or uses the saved board, runs the durable session pipeline, and exposes the common catalog and live state behavior

#### Scenario: Guest native connection

- **WHEN** any native host receives guest connection intent
- **THEN** the Rust backend exposes live telemetry while creating no saved device, profile write, controller hint, session row, location row, catalog revision, or durable ride metadata

#### Scenario: Guest reconnect and recreation

- **WHEN** a guest connection reconnects or its WebView is recreated
- **THEN** the Rust backend retains guest mode and bounded live state without promoting in-memory data to persistence

#### Scenario: Native policy coverage

- **WHEN** a future durable write is added to the connection or telemetry lifecycle
- **THEN** it has one explicit Rust-core saved/guest decision and cannot be reached from guest mode accidentally

### Requirement: Mirrored native BMS ownership

The Rust backend SHALL own connection-wide VESC request scheduling, common and hardware-specific firmware identity parsing, auxiliary-node discovery, bounded BMS values decoding, and slow BMS polling. Application coordination SHALL remain above protocol routing and the bounded Bluetooth port, and no BMS protocol owner SHALL depend on loopback transport, frontend, persistence, host interoperability, or view code.

#### Scenario: Cross-platform source layout

- **WHEN** a maintainer traces BMS discovery or decoding from any host
- **THEN** one Rust owner is available under the core application and VESC protocol crates with platform adapters limited to byte transport

#### Scenario: One write owner

- **WHEN** controller and BMS work both need the VESC stream
- **THEN** the Rust backend routes typed requests through one connection-wide scheduler rather than host polling timers

#### Scenario: Hand-written contract mapping

- **WHEN** BMS state is serialized for the frontend
- **THEN** the Rust backend explicitly maps every bounded contract field with common null, unit, array, and status semantics and no generated wire API layer

### Requirement: Mirrored native Refloat configuration ownership

The Rust backend SHALL own custom-schema chunk assembly, bounded Qt/zlib and XML decoding, normalized schema validation, CRC32C signature construction, typed value encoding/decoding, configuration operation lifecycle, and exact-version saved-configuration persistence. Protocol owners SHALL depend only on VESC framing/routing and configuration models; one application mutation coordinator SHALL own authorization, exclusivity, and scheduler submission for semantic controls, configuration writes, and Tune apply. Storage SHALL own schema cache and Tunes; loopback serialization SHALL expose only normalized bounded values. Frontend and platform hosts MUST NOT parse device XML, compute wire signatures, choose routes/commands, or persist authoritative Tunes.

#### Scenario: Locate schema codec

- **WHEN** a developer changes ConfigParams parsing or a VESC value encoding
- **THEN** one focused Rust protocol owner and shared golden vectors are available under the core Refloat/VESC crates

#### Scenario: Disposable WebView

- **WHEN** the frontend is recreated during schema loading or after saved configurations exist
- **THEN** Rust connection, operation, cache, and durable Tune ownership remain intact and are recovered through authenticated contract methods

#### Scenario: Shared flywheel boundary

- **WHEN** any host observes flywheel mode
- **THEN** the Rust coordinator invalidates configuration current state and rejects every device mutation except eligible flywheel disable

#### Scenario: Storage schema change

- **WHEN** exact-version saved configurations change during test-stage development
- **THEN** canonical catalog SQL and the one Rust store change together, the storage version is bumped, and incompatible local storage is recreated rather than migrated

#### Scenario: Compression dependency

- **WHEN** the Rust implementation decodes the compressed schema
- **THEN** it uses one bounded core compression implementation proven by the target build matrix

## REMOVED Requirements

### Requirement: One shared Apple backend

**Reason**: The Apple-only Swift implementation is replaced by the portable Rust backend used by Apple and Android.

**Migration**: Rewrite shared behavior under `/backends/core`, make both Apple hosts consume its C ABI and native library, retain only Apple adapters/wrappers under `/backends/apple`, and delete the Swift backend after Apple cutover gates pass.

### Requirement: No generic shared-code directory

**Reason**: Deliberate shared Rust implementation is now the mechanism that prevents four backend copies; the previous prohibition encoded the duplication this change removes.

**Migration**: Keep immutable SQL/fixtures in their existing shared locations, move executable shared ownership to focused crates under `/backends/core`, and prohibit unrelated utility dumping through the new portable-backend boundaries.
