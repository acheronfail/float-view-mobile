## MODIFIED Requirements

### Requirement: Durable detailed telemetry lifecycle

For a commands-31/32 package, each accepted compact response SHALL replace the latest compact cache and its `slowObservedAt`; the next accepted realtime response SHALL merge that cache into one normalized sample. For Refloat 1.3.0, each accepted slow command-33 response SHALL replace the latest selectable slow cache and its `slowObservedAt`; the next accepted fast command-33 response SHALL merge that cache into one normalized sample. A command-10 mode-0 response for 1.3.0 SHALL update only current active firmware-fault state and MUST NOT replace typed or extension telemetry. Device-reported accumulated values SHALL replace previous values even when they decrease. Each accepted normalized sample SHALL be assigned one monotonic session sequence and stored durably with bounded typed detail and extension values in the same commit unit. A changed discrete state SHALL append one deduplicated typed `session_event` in that transaction. Live state, session metadata/data pages, and export SHALL derive from the normalized records and observed events. Apple and Android SHALL share baseline 100 storage semantics: recognised pre-baseline development data may reset once, while baseline and subsequent supported telemetry require preservation and explicit tested upgrades for incompatible changes.

#### Scenario: Persist and recover detailed sample

- **WHEN** a detailed sample is accepted and the frontend is recreated
- **THEN** the live snapshot and session metadata/data recover the same committed values and sequence without requiring the WebView to have observed the original event

#### Scenario: Selectable slow merge

- **WHEN** an accepted Refloat 1.3.0 slow response is followed by a current-generation fast response
- **THEN** the resulting sample contains the latest slow typed and extension values with their `slowObservedAt` and one new monotonic sample sequence

#### Scenario: Device counter decreases

- **WHEN** a compact or selectable slow response reports an odometer or energy total lower than the latest cached value
- **THEN** the backend accepts the new finite protocol-valid value, records its new `slowObservedAt`, and does not clamp or reconstruct the old value

#### Scenario: Partial storage failure

- **WHEN** durable storage cannot commit a detailed sample
- **THEN** no partially populated sample is exposed as committed and the existing bounded failure and session-integrity rules apply

#### Scenario: Detailed export

- **WHEN** a session containing detailed and extension telemetry is exported
- **THEN** the export contains stable typed names, bounded extension values, nulls for unavailable fields, and no private Bluetooth or controller identifier
