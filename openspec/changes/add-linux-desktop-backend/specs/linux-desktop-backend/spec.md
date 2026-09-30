# Spec Delta

## Purpose

Provide a Linux desktop application with real Bluetooth and the shared Float View frontend, preserving portable backend behavior while exposing Linux capability and lifecycle limits honestly.

## ADDED Requirements

### Requirement: Linux desktop uses the shared application service

The Linux desktop application SHALL provide the existing frontend and canonical authenticated backend behavior through one process-owned shared service. It SHALL use the existing shared product persistence, firmware support, BLE policy, mutation ordering and state publisher. Desktop views and transport clients SHALL remain disposable. Linux MUST NOT provide a parallel frontend API, product service, protocol implementation or platform-specific frontend.

#### Scenario: Product workflow on Linux

- **WHEN** a user connects a supported board and uses saved boards, live telemetry, controls, Tunes, recording, history or archives
- **THEN** the existing shared validation, results, errors and durable behavior apply, except for explicitly unavailable OS capabilities

#### Scenario: Frontend recreation

- **WHEN** the WebView is replaced or a frontend client reloads while recording
- **THEN** the same service and session continue and the new client receives complete non-regressing authoritative state without replaying accepted mutations

### Requirement: Linux desktop startup and presentation

Linux SHALL display the shared embedded frontend in a native desktop window using the assigned loopback port, current authentication and exact Origin validation. It SHALL prevent navigation from replacing the app with an untrusted remote document and SHALL show actionable startup failures. Native surfaces SHALL follow the existing saved/system appearance and canvas behavior without a fixed splash delay.

#### Scenario: Desktop launch

- **WHEN** the application starts with its prerequisites available
- **THEN** one native window loads the embedded frontend from `127.0.0.1:11013` with the current launch authentication context and additive `linux` bootstrap platform metadata; desktop exports use Save labeling

#### Scenario: Startup failure

- **WHEN** the port is occupied, the data root cannot be opened, or the embedded frontend is missing
- **THEN** startup fails visibly without switching ports, deleting existing data or replacing the real backend with a mock

#### Scenario: Untrusted navigation and client

- **WHEN** remote document navigation, an unsolicited popup or a protected request with an invalid token or Origin is attempted
- **THEN** the operation is rejected without granting access to app state or expanding the loopback boundary

### Requirement: Linux Bluetooth executes bounded shared policy

Linux SHALL execute real OS Bluetooth discovery, connection, profile discovery, notification setup and ordered writes for the existing supported UART capabilities. Shared policy SHALL decide device eligibility, write mode, segmentation, deadlines, mutation admission and reconnect. Native operations and event queues MUST be bounded and qualified by service lifetime, connection generation and operation identity. No native execution path SHALL replay uncertain device writes.

#### Scenario: Connect a discovered or saved board

- **WHEN** a discovered eligible device or valid private saved association is selected
- **THEN** Linux supplies observed capabilities and establishes the required notifications before shared policy publishes readiness
- **AND** a saved association does not bypass profile validation

#### Scenario: Write flow control

- **WHEN** a frame needs multiple chunks or the OS delays completion
- **THEN** chunks preserve shared ordering and payload bounds, at most one is in flight, and no-response submission is not reported as remote acknowledgement

#### Scenario: Stale callback after reconnect

- **WHEN** an operation from a previous connection completes after reconnecting to the same device
- **THEN** it cannot publish readiness, deliver bytes into the replacement connection, advance its write queue or repeat a mutation

#### Scenario: Saturation or timeout

- **WHEN** setup stalls, write progress stops or native event capacity is exhausted
- **THEN** the affected work fails within its documented bound, cleanup remains possible, and shared policy receives the failure without silent byte loss or unbounded resource growth

#### Scenario: Multiple or removed adapters

- **WHEN** several adapters are present or the selected adapter disappears
- **THEN** selection is deterministic and a live connection is not silently moved to another adapter; loss invalidates its work and reconnect decisions remain shared

### Requirement: Linux capability reporting reflects actual OS support

Linux SHALL distinguish unavailable Bluetooth hardware, disabled radio, unavailable Bluetooth service and denied OS access through existing canonical state. It SHALL expose only implemented permission recovery actions, operate without requiring root or installing system policy, and refresh observations when OS state changes. The initial Linux host SHALL report enabled live location capture and background capture as unavailable, while preserving the canonical user-disabled state and imported location history.

#### Scenario: Bluetooth blocked externally

- **WHEN** Bluetooth hardware is absent, the radio is off, the OS service is unavailable or access is denied
- **THEN** the application remains usable for offline product/history operations, reports the corresponding observed condition, and does not present a fabricated permission prompt or successful connection

#### Scenario: Bluetooth becomes available

- **WHEN** external OS changes restore the selected Bluetooth capability
- **THEN** the application refreshes canonical state and lets shared policy proceed without restarting the frontend or introducing an adapter-owned retry loop

#### Scenario: Optional location unavailable

- **WHEN** location recording is enabled on Linux
- **THEN** location reports unavailable capture with no unsupported recovery action, Bluetooth/telemetry recording remains usable, and no invented location samples are stored

#### Scenario: Review imported locations

- **WHEN** a valid imported session includes location samples
- **THEN** the existing history and review flows can read them regardless of Linux live acquisition support

### Requirement: Desktop lifetime preserves durable work and bounds shutdown

Minimizing the desktop window SHALL preserve the process-owned service and permitted recording. Closing the window or explicitly quitting SHALL stop the desktop process with bounded cancellation and durable cleanup. Suspend/resume SHALL be treated as a possible transport interruption, with stale work invalidated and reconnect governed by shared policy. Linux SHALL NOT promise recording during system sleep or after process exit.

#### Scenario: Minimize during recording

- **WHEN** the window is minimized while a healthy board is recording
- **THEN** Bluetooth and durable session progress continue independently of frontend visibility

#### Scenario: Suspend and resume

- **WHEN** the machine suspends during a connection and later resumes
- **THEN** current OS state is refreshed, expired or old-generation work cannot be replayed, gaps contain no invented samples, and shared reconnect policy controls recovery

#### Scenario: Quit under load

- **WHEN** the user quits while Bluetooth, timers, a file dialog or shared maintenance is active
- **THEN** shutdown cancels owned operations and releases resources within documented bounds without acknowledging unfinished work as successful

#### Scenario: Interrupted shutdown

- **WHEN** the OS terminates the application before cleanup completes
- **THEN** the next launch uses existing shared recovery to retain committed data without treating interruption as a format mismatch

### Requirement: Linux storage and file presentation preserve shared semantics

Linux SHALL use a private per-user persistent root and the shared exclusive-owner lock. It SHALL use the current storage and archive formats without Linux migrations or alternate readers. File presentation SHALL retain service-issued leases until the selected copy completes or presentation is cancelled. Normal diagnostics MUST NOT expose tokens, Bluetooth addresses, private identities or raw telemetry.

#### Scenario: Concurrent launch

- **WHEN** another instance owns the data root or assigned port
- **THEN** the new launch activates the existing application or fails clearly without creating a second storage owner, killing the other process or resetting its data

#### Scenario: Export cancellation or copy failure

- **WHEN** the user cancels an export destination or copying fails
- **THEN** the operation completes with its actual outcome and releases its lease once without deleting source session data or reporting success

#### Scenario: Corrupt or inaccessible storage

- **WHEN** opening current-format data fails because of corruption or an operational error
- **THEN** the application preserves it and reports failure; only explicit incompatible owned development format markers permit reset

### Requirement: Linux builds and acceptance use canonical tooling

Linux SHALL provide documented source-build, native run and focused test entry points using pinned Rust dependencies and explicit system prerequisites. Its canonical target SHALL reuse the existing conformance scenarios with isolated test storage and explicit simulation selection. Acceptance SHALL distinguish shared/software behavior from real Linux OS Bluetooth and desktop observations. Missing required physical evidence MUST remain incomplete rather than be represented by simulated results.

#### Scenario: Build and run on Linux

- **WHEN** a developer uses `just build-linux`, `just run linux` or `just test linux` with documented prerequisites
- **THEN** the relevant shared frontend/Linux artifacts build or execute without requiring Apple/Android toolchains or fetching toolchains implicitly

#### Scenario: Canonical software target

- **WHEN** canonical Linux software conformance runs
- **THEN** the selected Linux host uses the production shared service with controlled OS inputs, shared assertions and isolated storage, covering authentication, product behavior, recovery, ordering and bounds

#### Scenario: Physical Linux checkpoint

- **WHEN** Linux is accepted against the programmable fixture through the production OS adapter
- **THEN** evidence covers discovery, saved/guest connection, notifications, supported write modes, segmented writes, telemetry/recording, reconnect, adapter/radio loss and desktop interruption recovery
- **AND** native WebView recovery, minimize/quit, theme and file presentation have desktop evidence, and unavailable required scenarios are identified explicitly
