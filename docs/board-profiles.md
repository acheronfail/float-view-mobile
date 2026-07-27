# Per-board profiles and local configuration

Status: product, data-model, and API specification for board-specific settings.

Many important board properties cannot be reliably discovered from VESC
firmware. Float View stores a versioned app-local Board Profile for each saved
board. Profiles improve recognition, live display, alerts, battery estimation,
review, and diagnosis without writing configuration back to the controller.

Every profile field is optional. Saving a device automatically creates a partial
profile, and the UI highlights useful missing configuration without blocking
connection or logging.

## Safety boundary

Board Profile settings are local Float View metadata. Editing them must not send
motor, firmware, app configuration, CAN configuration, BMS configuration, or
other persistent writes to the VESC.

Read-only discovery may inspect firmware, hardware type, CAN peers, BMS presence,
and other supported values. Any future “apply to controller” feature is a
separate privileged capability requiring its own safety design and confirmation.

## Profile sections

### Identity

- user-facing display name;
- optional notes;
- optional board make/model;
- versioned visual identity descriptor;
- creation/update timestamps.

The visual descriptor supports a later shared-frontend board/icon builder. The
backend stores a small versioned JSON document but does not render it. Initial
fields can cover:

- board/deck silhouette identifier;
- rail style identifier;
- front/rear/side colors;
- wheel/tire color or style;
- accent color;
- optional labels.

The frontend owns the builder, preview, accessibility, and responsive rendering.
Arbitrary image import and sync are later decisions.

### Battery

- cell chemistry/type or named discharge-curve identifier;
- series cell count (`S`);
- parallel cell count (`P`);
- optional cell capacity in amp-hours;
- optional pack capacity override;
- optional nominal/minimum/maximum per-cell voltage;
- voltage correction in volts;
- battery-estimation model/version.

Pack topology is represented explicitly as `seriesCount` and `parallelCount`,
not a single ambiguous string. The UI may display forms such as `20S2P`.

Begin with one generic configurable cell model. Named chemistry/cell discharge
models require a later research and validation pass.

Voltage-only state of charge is an estimate, especially under load. The UI and
API label it as estimated. When a VESC VBMS provides state of charge, use that
reported value as primary while retaining the voltage-based estimate for
comparison. Canonical samples preserve raw observed voltage; the frontend
calculates corrected voltage and estimates from the immutable profile snapshot.

### Motor and board hardware

- motor variant/catalog identifier;
- optional free-form motor label;
- rail variant/catalog identifier;
- optional free-form rail label;
- wheel/tire variant where useful later;
- relevant drivetrain constants needed for speed/distance conversion;
- optional board-specific notes.

Product catalogs are shared-frontend assets where possible. The backend stores
stable identifiers and user-entered values; it does not contain presentation
catalogs or vendor imagery.

The initial motor catalog contains Hypercore (XR) and Superflux HS plus a
free-form motor entry. Additional motors and rail variants are a researched
post-release catalog expansion.

### Controller and CAN

- primary motor-controller CAN ID;
- whether the CAN ID uses automatic discovery or a manual override;
- discovered CAN peers with hardware/firmware identity where available;
- selected primary controller when more than one candidate exists;
- discovery timestamp and ambiguity/error state.

The Bluetooth peripheral may be a VESC Express that forwards commands to the
primary motor VESC. Transport identity and `controllerCanId` are separate;
selecting the Express must never silently make its own identity the motor
controller ID.

The backend attempts read-only CAN discovery after connection when supported. A
single clear candidate can become the discovered value. Multiple candidates are
presented to the user; the backend must not silently choose an ambiguous primary
controller. A manual override remains effective until the user returns that
field to automatic mode.

### VESC VBMS

- presence mode: `auto`, `enabled`, or `disabled`;
- discovered presence and capability;
- whether it is CAN-enabled;
- optional BMS CAN ID/role;
- supported BMS field/cell count discovered at runtime;
- manual override and discovery metadata.

`enabled` means Float View should attempt the supported read-only BMS path; it
does not write BMS configuration.

The first release supports VESC VBMS only. Other smart-BMS families are
`unsupported` rather than treated as equivalent CAN devices.

### Calibration

- pack voltage correction in volts;
- optional speed/distance correction factor;
- other future sensor corrections with explicit units and conservative bounds.

Always store the raw observed value plus the profile revision and correction
setting needed to reproduce presentation. Corrections are profile data, not
destructive sample columns. The frontend calculates corrected display/review
values. Native alert evaluation may transiently apply the same versioned
correction but must persist the raw observation and rule/profile snapshot.

## Discovered, configured, and effective values

Every setting that may be discovered or overridden uses the same conceptual
shape:

```text
mode: auto | manual
discoveredValue: optional
configuredValue: optional
effectiveValue: optional
source: discovered | manual | default | unavailable
discoveredAt: optional UTC timestamp
```

Resolution:

1. A valid manual value wins while `mode` is `manual`.
2. A current valid discovered value wins in `auto`.
3. A documented safe/default value may be used only when the field defines one.
4. Otherwise the field is unavailable and dependent features say so explicitly.

Discovery never overwrites a manual value. It can update `discoveredValue` so
the UI can show that the board differs from the override. Returning to `auto`
selects the latest valid discovery.

Profile updates use optimistic concurrency through `profileRevision`. A stale
update returns a structured conflict with the current revision rather than
silently overwriting changes from another frontend instance.

## Validation

JSON Schema owns field types, bounds, units, and enum/catalog identifiers.
Minimum validation includes:

- plausible positive `seriesCount` and `parallelCount` bounds;
- compatible per-cell minimum/nominal/maximum ordering;
- bounded voltage and speed/distance corrections;
- finite numeric values only;
- valid CAN ID range and no implicit conversion from display strings;
- known chemistry/model identifiers or an explicit custom model;
- visual descriptor size, version, and color/identifier limits;
- cross-field warnings when discovered cell count conflicts with a manual
  topology or BMS report.

Validation errors name stable fields/codes. The frontend owns friendly guidance.
Potentially surprising but valid overrides require confirmation in the UI and
remain marked as manual.

## Derived battery and board values

The normalized native pipeline keeps raw pack voltage, raw speed/distance inputs,
and the profile revision/model used. A VBMS-reported state of charge is an
observed field, not a replacement for raw voltage.

The frontend calculates corrected voltage, speed/distance, and voltage-derived
battery percentage from the effective series count, generic model, correction,
and immutable snapshot. Model upgrades are versioned so old sessions remain
explainable. Native summaries or alerts that require a derived value use the
same shared fixtures and record their model/profile revision.

Derived metrics needed only for charts may be calculated in the shared frontend.
Derived metrics required for background alerts, adaptive behavior, or durable
session summaries use a small native implementation generated or tested against
shared fixtures.

## Persistence model

The catalog adds:

```text
board_profiles
  board_profile_id
  saved_device_id
  profile_revision
  display_name
  profile_json
  created_at
  updated_at

board_discoveries
  discovery_id
  saved_device_id
  observed_at
  firmware_identity
  hardware_identity
  can_topology_json
  bms_capabilities_json
  discovery_status
```

Structured/indexed fields used frequently, such as display name and profile
revision, remain catalog columns. Versioned or evolving settings live in a
schema-validated profile document so adding a visual descriptor or new hardware
catalog field does not require a column for every UI option.

Sensitive platform reconnect bindings remain outside the profile document.

## Session snapshots

At automatic session creation, copy the full effective Board Profile into the
per-session database with:

- `boardProfileId` and `profileRevision`;
- discovered/configured/effective values and sources;
- battery-estimation model and inputs;
- firmware/hardware/CAN/BMS discovery snapshot;
- display name and visual descriptor;
- all corrections and units.

The session catalog also copies the display name and historical device identity
needed for fast list/filter presentation.

Changing a Board Profile never rewrites completed sessions. If a profile changes
while a session is active:

1. validate and persist the new profile revision;
2. append a `board_profile_changed` session event;
3. copy the new snapshot into the active session;
4. tag subsequent derived samples with the new revision;
5. preserve raw sensor values so analysis can recompute alternatives later.

## Product API

- `boardProfiles.get({ savedDeviceId })`
- `boardProfiles.update({ savedDeviceId, expectedRevision, patch })`
- `boardProfiles.resetFields({ savedDeviceId, expectedRevision, fields })`
- `boardProfiles.refreshDiscovery({ savedDeviceId })`
- `boardProfiles.getDiscovery({ savedDeviceId })`

Events:

- `boardProfiles.changed`
- `boardProfiles.discoveryChanged`

`savedDevices.list/get` include the effective display name and visual descriptor
needed by the Device tab, plus a concise completeness/warning state. Full profile
documents use `boardProfiles.get`.

The frontend contains the motor, rail, chemistry, and visual catalogs. Catalog
version/identifier is stored in the profile so unknown future entries are
preserved rather than erased by older clients.

## Product experience

Board Settings is reachable from:

- each saved-device card;
- the connected Live view;
- the post-connect setup prompt when required information is missing.

Sections are Identity, Battery, Motor/Drivetrain, Rails/Appearance,
Controller/CAN, Smart BMS, and Calibration. Each discoverable field shows
`Automatic`, `Manual`, `Discovered`, `Conflict`, or `Unavailable` status.

Connection and automatic logging do not require every optional field. Features
degrade explicitly:

- unknown battery topology means no trustworthy percentage estimate;
- unknown motor/drivetrain constants may limit speed/distance accuracy;
- ambiguous CAN discovery requires user selection for targeted controller data;
- unknown BMS capability hides or marks BMS-only values unavailable.

The UI encourages completion without blocking safe basic logging.

## Verification

Shared schema and behavior tests cover:

- first-connect profile creation with partial discovery;
- manual value precedence and return to automatic mode;
- discovery changes that do not overwrite overrides;
- optimistic-concurrency conflicts;
- battery S/P validation and voltage correction;
- battery percentage fixtures for each supported chemistry/model;
- raw versus corrected value preservation;
- single, missing, and ambiguous CAN discovery;
- smart-BMS auto/enabled/disabled behavior;
- rename and visual-descriptor round trips;
- unknown future catalog identifiers;
- session snapshot immutability after profile changes;
- mid-session profile revision events and derived-value tagging;
- forgetting a device while preserving session snapshots.

Hardware tests compare discovered firmware, CAN topology, and BMS presence with a
declared fixture profile. They remain read-only.
