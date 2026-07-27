# First-release implementation scope lock

Status: accepted product and platform decisions for implementation.

Last updated: 2026-07-28.

This document resolves the planning questions raised before the backend reset.
It is the scope lock for the first large implementation pass and the first
store-distributed release. When an older planning statement conflicts with this
document, this document wins unless a newer accepted decision or committed API
schema explicitly supersedes it.

## Delivery boundary

The first large implementation pass completes R0 through R5 and creates the R6
native Bluetooth adapter scaffolds. It must leave all three hosts runnable
against deterministic fake Bluetooth, with frozen API schemas, matching Swift
and Kotlin protocol behavior, durable native logging, shared black-box
conformance, and the first Device/Live/Sessions/Review/Diagnosis UI journeys.

R6 through R8 then connect those tested cores to real CoreBluetooth and Android
GATT, prove lifecycle behavior, and complete repeated hardware acceptance.

The first public release is distributed through the Apple App Store and Google
Play Store. Linux and Windows are planned later and are not release blockers.

The project license is not yet final. AGPL-3.0 is the leading choice, but no
release, third-party code incorporation, or public license claim may assume that
choice until it is explicitly accepted. Independent implementation and
fixture-based behavioral work can proceed.

## Supported platform baseline

| Platform | Minimum             | Required verification destinations                                                                 |
| -------- | ------------------- | -------------------------------------------------------------------------------------------------- |
| iOS      | iOS 17              | Physical devices on iOS 17, iOS 18, and the latest iOS 26 release available to the build toolchain |
| Android  | Android 10 / API 29 | Physical Samsung Galaxy S9 on Android 10 plus a current supported Android test destination         |
| macOS    | macOS 14            | Apple-silicon, BLE-capable Mac                                                                     |

The Android application may compile against a newer SDK while retaining API 29
as its minimum. App Store and Play policy checks use the current submission
requirements at release time.

## Supported device and firmware baseline

Initial hardware acceptance covers:

- Thor300;
- Thor301;
- Floatwheel XRV;
- Floatwheel PintV;
- a VESC Express transport where it forwards Bluetooth requests to the primary
  motor controller;
- VESC VBMS as the only smart-BMS family.

The first implemented and verified protocol profile is:

- VESC firmware 6.06;
- Refloat package 1.2;
- Refloat ride state as the authoritative riding/not-riding signal.

The intended compatibility floor is VESC firmware 6.02 and newer. After the
6.06/Refloat 1.2 slice passes, a dedicated compatibility pass adds fixtures,
version branches, and hardware evidence for 6.02 through 6.05 and representative
Refloat releases from the preceding several years.

Float package firmware is explicitly unsupported. Float View supports Refloat
only. Unknown firmware/package combinations must report an honest
`unsupported_firmware` or reduced capability rather than silently using the
closest decoder.

## Product behavior locked for v1

### Connections and sessions

- Exactly zero or one board can be actively connecting/connected. This is a
  permanent product constraint, not merely a v1 capacity limit.
- Saved devices, discoveries, and historical sessions remain collections.
- Connection is always initiated by an explicit user action; saved boards do not
  auto-connect at launch.
- Reaching `ready` durably creates a Session and LoggingJob before normal Live
  mode is shown.
- Storage startup failure blocks ordinary Live mode. An explicitly labelled
  debug/test capability may connect without recording in non-release workflows.
- Multiple riding intervals during one continuous connection remain one Session.
- Unexpected connection loss starts bounded exponential reconnection for the
  active Session. Explicit disconnect never reconnects.
- Reconnection defaults to five minutes and is user-configurable from disabled
  through a backend-advertised maximum, initially 60 minutes. Every gap and
  attempt is persisted.
- Force-quit makes the active Session `interrupted`; the app never represents
  unobserved time as logged.
- macOS window close leaves the native owner visibly running. Explicit Quit
  flushes the log and disconnects cleanly.

### Optional session linking

Users may opt into automatically linking adjacent Sessions from the same
`deviceHistoryId` when their gap is within a configured time window. This lets a
day or journey span a charging break without rewriting or merging the underlying
session databases.

- default: disabled;
- suggested initial window when enabled: two hours;
- configurable range: 1 minute through 24 hours;
- only completed/interrupted Sessions for the same historical device can link;
- links are catalog metadata and are reversible;
- each member Session retains its own identity, integrity, firmware/profile
  snapshot, start/end state, and exportability;
- the Sessions UI presents a linked group as one expandable entry with segment
  boundaries and aggregate summaries;
- review overview data can be combined in the shared frontend from bounded member
  queries; lossless Diagnosis always identifies the concrete member Session.

### Sampling and logging

- Native code uses Refloat ride state to select sampling mode.
- While idle, it probes ride state at 2 Hz but persists ordinary samples at 1 Hz.
- While riding, it starts at 10 persisted samples per second.
- It retains the riding rate for 15 seconds after leaving riding state.
- Rates, tail duration, and reconnection duration are user-configurable within
  backend-advertised safe limits.
- Raw observations are canonical. In particular, raw pack voltage is always
  stored with the Board Profile revision and correction settings required to
  reproduce display values.
- Voltage correction and other presentation estimates are calculated in the
  shared frontend. A small native evaluator may apply the same versioned
  correction transiently for background alerts, but it may not overwrite raw
  evidence.
- Optional raw packet capture is disabled by default and has separate byte/time
  caps when explicitly enabled for diagnosis.

### Alerts

The first rule set supports:

- high duty cycle;
- high and low pack voltage;
- high motor temperature;
- high controller temperature;
- individual and combined footpad state transitions;
- connection loss.

Rules support shared, testable threshold/edge, duration/debounce, hysteresis, and
cooldown semantics as appropriate. Native output may use visual presentation,
sound, haptics, and background notifications subject to OS/user settings. Alerts
are advisory only and never write configuration or affect motor control.
Acknowledgement clears the current occurrence; disabling a rule is a separate
action. Storage/logging failures remain mandatory system notices rather than
user-authored alert rules.

### Board Profiles

Every field is optional. Float View auto-creates a partial profile when a device
is saved and clearly identifies missing settings without blocking logging.

- battery: begin with a generic configurable cell model; research and add
  validated named discharge models later;
- topology: optional series and parallel counts plus capacity inputs;
- VBMS: use reported VBMS state of charge as the primary estimate when available
  and retain voltage-based estimation for comparison/audit;
- motors: initially provide Hypercore (XR) and Superflux HS plus a free-form
  entry;
- controller: `controllerCanId` identifies the primary motor VESC, not the VESC
  Express Bluetooth transport;
- BMS: support VESC VBMS only;
- visual identity: implement the eventual board/icon builder in shared Svelte
  and SVG, never native UI;
- corrections: preserve raw measurements and immutable profile snapshots.

### Data ownership and storage

- App-private SQLite storage is sufficient for first release. Data survives
  crashes, reboot, and updates; uninstall may remove it.
- Exclude log databases from OS/cloud backup initially.
- Never automatically delete completed Sessions.
- Warn below 1 GiB available, show a prominent warning below 500 MiB, and
  cleanly stop logging below 250 MiB. Capabilities report effective thresholds.
- Session deletion is included, requires confirmation in the shared UI, and is
  idempotent at the backend.
- First-release exports are a lossless native Float View archive and CSV.
- Import from Floaty, Float Control, VESC Tool, or other formats is post-release.

### Frontend and native boundary

- Product charts and interactions use custom shared Svelte/TypeScript/SVG code;
  no charting framework is required initially.
- There is no analytics SDK in the first release.
- Each platform serves the shared frontend over an authenticated ephemeral
  loopback HTTP/WebSocket server. A small audited server dependency is allowed
  when it materially improves protocol correctness, but transport behavior must
  remain conformant and the dependency decision must be recorded.
- Diverging from HTTP/WebSocket on one platform requires a concrete measured
  platform limitation and an accepted architecture decision.
- Tear down the mobile WebView 30 seconds after backgrounding, or immediately
  under memory pressure. Native logging and alerting remain the owners.

## iOS background validation

iOS background logging is an empirical acceptance target, not an assumption.
The implementation uses `bluetooth-central`, CoreBluetooth state restoration,
native persistence, and no WebView-owned timer.

For iOS 26 and later, implement a visible logging Live Activity when an active
session starts if hardware testing confirms it provides the documented
CoreBluetooth foreground-equivalent privileges needed by the protocol. Test it
with the manager instantiated and the Live Activity started before the app
backgrounds.

For iOS 17 and 18, verify CoreBluetooth background/restoration behavior directly
and report its actual supported sampling rate and gaps. A Live Activity may still
provide useful visible session status, but it must not be credited with logging
accuracy without evidence on that OS.

Location recording is deferred until after the first release. Background
location permission must not be used merely to keep the process alive. A later
ride-location feature may request it only with a clear user benefit, disclosure,
permission flow, retention policy, and App Store-compliant background use.

## Explicit first-release exclusions

- Float package support;
- configuration writes, firmware updates, and motor-affecting commands;
- more than one active board;
- Linux and Windows hosts;
- GPS, route maps, or background location;
- imports from other applications;
- automatic cloud backup or sync;
- automated causal diagnosis;
- native product/chart UI;
- analytics SDKs.

## Post-release implementation tracks

1. **Compatibility:** qualify VESC 6.02-6.05 and prior Refloat releases with
   versioned fixtures, VESC Tool comparisons, and hardware profiles.
2. **Ride location:** add opt-in GPS capture, map review, privacy/retention
   controls, and truthful iOS background verification.
3. **Battery models:** research validated chemistry/cell discharge curves and
   versioned estimation models.
4. **Import:** define provenance-preserving importers for Floaty, Float Control,
   VESC Tool, and other documented formats.
5. **Board visuals:** ship the Svelte/SVG board builder and grow catalogued
   motors, rails, and board variants.

## Remaining gates

Only these choices remain intentionally open:

1. accept the repository and distribution license (AGPL-3.0 is the current
   candidate);
2. record the exact physical device/OS/build identifiers used for release
   qualification;
3. select representative older Refloat versions during the post-6.06
   compatibility pass.

These do not block R0-R5 architecture work. The license blocks incorporating
code whose terms depend on the final choice and blocks public distribution.
