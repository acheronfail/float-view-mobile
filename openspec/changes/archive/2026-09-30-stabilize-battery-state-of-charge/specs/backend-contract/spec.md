## MODIFIED Requirements

### Requirement: Authoritative battery estimate projection

The authenticated snapshot SHALL expose a bounded cell-profile ID/label catalog and one shared-service battery estimate for the active connection. The estimate SHALL identify its nullable fraction, selected profile, resolved series count, sample sequence, profile revision, catalog version, calculation mode, and closed unavailable reason. Fraction SHALL be null when unavailable and finite in [0,1] otherwise. The estimate SHALL additionally expose `method` as `voltage_smoothed`, `charge_integrated`, or null, and boolean `provisional`. Unavailable results SHALL use null method and false provisional; available initial or uncertain estimates SHALL be provisional. Method SHALL describe the current temporal update, while mode describes the voltage observation. Modes SHALL be `load_compensated`, `load_clamped`, `voltage_only`, or `unavailable`; unavailable reasons SHALL be `not_configured`, `series_mismatch`, `missing_voltage`, or `not_applicable`. Apple, Android, frontend mock, and canonical conformance SHALL share these semantics. Raw `LiveSample.batteryLevel` and existing recorded/exported telemetry SHALL retain their meanings. Estimation SHALL NOT introduce BLE polls or bypass saved-session persistence before publication.

#### Scenario: Saved sample publication

- **WHEN** a saved board produces a new accepted telemetry sample
- **THEN** its sample and matching estimate appear together only after the existing persistence gate succeeds in a complete monotonic snapshot

#### Scenario: Guest or BMS connection

- **WHEN** an active connection is a guest or BMS-only device
- **THEN** its board estimate has null fraction and reason `not_applicable` without creating durable profile state

#### Scenario: Raw telemetry retained

- **WHEN** a controller reports a battery fraction differing from the calculated estimate
- **THEN** both values remain distinguishable and the recorded controller field is unchanged

### Requirement: Battery estimation profile validation

Saved profile requests and imported profiles SHALL validate optional `batteryCellProfileId` against the bounded catalog and optional `batteryMinimumCellVoltageV` against [2.7,4.2), and optional `batteryCapacityAh` as a finite number in [0.1,200], alongside existing series/parallel validation. Optional nullable `batteryPresetId` SHALL reference the bounded preset catalog. Preset selection and manual overrides SHALL follow the atomic selection requirements; imported records SHALL retain valid overrides and require any non-null identity to match their S/P/profile values. Capacity SHALL mean full pack Ah before display-empty remapping; absent or null capacity SHALL select voltage smoothing without marking configuration incomplete. Updates SHALL retain existing revision conflict behavior and SHALL NOT write controller settings. Absent estimation fields SHALL NOT change existing profile-completeness policy or infer a cell identity.

#### Scenario: Invalid selection or voltage

- **WHEN** a save supplies an unknown profile ID, nonfinite minimum, or minimum outside the allowed range
- **THEN** the request fails with the existing invalid-request error and leaves saved configuration and live estimate unchanged

#### Scenario: Conflicting update

- **WHEN** a battery-profile save uses a stale revision
- **THEN** it fails with the existing conflict behavior without publishing an estimate based on the rejected values

#### Scenario: Archive round trip

- **WHEN** a valid configured board is exported and imported
- **THEN** its preset identity, selected profile, empty voltage, and optional capacity including manual overrides round trip through the existing profile mechanism, without replaying preset defaults, and invalid or inconsistent values are rejected


#### Scenario: Invalid capacity

- **WHEN** a save or import supplies a nonnumeric, non-finite, or out-of-range capacity
- **THEN** it is rejected without changing saved configuration or estimator state

#### Scenario: Capacity cleared

- **WHEN** a valid capacity is cleared through a revisioned profile update
- **THEN** charge integration is disabled and the next estimate is coherently reseeded using voltage smoothing and the accepted revision

## ADDED Requirements

### Requirement: Bounded battery preset catalog

Authenticated snapshots SHALL expose a versioned catalog of at most 128 uniquely identified battery variants with friendly label, manufacturer, cell label, S/P counts, resolved cell-profile ID, and optional nominal full-pack Ah. IDs and version SHALL be bounded to 64 characters and labels to 128; counts and capacity SHALL obey existing validation limits. A preset without a dedicated cell curve SHALL resolve to the existing generic profile. The catalog SHALL NOT change saved configuration on snapshot reads or infer installed hardware from controller identity, voltage, or a matching electrical tuple. Apple, Android, and the frontend mock SHALL expose equivalent metadata and selection behavior. Out-of-range packs SHALL NOT be offered as selectable presets.

#### Scenario: Known and generic curve presets

- **WHEN** a client reads the catalog
- **THEN** ADV2 resolves to 20S2P, Samsung 50S profile, and 10 Ah nominal capacity, while X7S P30B resolves to 20S2P, generic profile, and 6 Ah nominal capacity
- **AND** manufacturer/product cell variants are distinguishable by name

#### Scenario: Bounds and unsupported pack size

- **WHEN** the preset catalog is projected
- **THEN** its IDs are unique, fields and collection remain bounded, every curve reference is valid, and the researched 40S1P pack is excluded from the 36S-limited contract

#### Scenario: No automatic reconfiguration

- **WHEN** a snapshot is read after a catalog update or reconnect
- **THEN** persisted identity, counts, curve, capacity, and empty voltage remain unchanged and no preset defaults are reapplied

### Requirement: Atomic battery preset selection and overrides

A non-null preset identity in a revisioned profile patch SHALL explicitly select that preset and atomically save identity, S/P counts, resolved profile, nominal capacity or null, and the default display-empty voltage of 3.0 V, preserving unrelated profile fields. Such a selection patch SHALL NOT also specify battery-value overrides; conflicting mixed input SHALL be rejected. Re-selecting the same preset SHALL restore defaults. Null identity alone SHALL clear only identity. Without an explicit selection, manual S/P/profile edits SHALL clear identity if the resulting values depart from the preset; capacity/empty-voltage overrides SHALL retain identity. Effective manual S/P changes SHALL clear prior capacity unless the same validated patch supplies an explicit replacement; curve-only edits SHALL retain capacity. Invalid IDs/values, revision conflicts, and persistence failure SHALL leave the entire saved configuration and estimate unchanged. An accepted selection SHALL produce at most one coherent estimator reconfiguration using the accepted revision. Preset identity alone SHALL NOT establish completeness or reset charge when effective estimation values are unchanged.

#### Scenario: Complete selection

- **WHEN** a valid selection request identifies a preset at the current profile revision
- **THEN** all battery defaults and identity are persisted together, unrelated profile fields remain intact, and no snapshot exposes a partially changed pack

#### Scenario: Mixed or unknown selection

- **WHEN** a request selects an unknown preset or combines non-null preset identity with battery-value overrides
- **THEN** it fails with the existing invalid-request error without modifying stored values or the estimate

#### Scenario: Failed or conflicting selection

- **WHEN** a preset selection uses a stale revision or persistence fails
- **THEN** the entire selection is rejected and the previous saved configuration and estimate remain authoritative

#### Scenario: Manual configuration and capacity retention

- **WHEN** a manual S/P/profile edit departs from the saved preset
- **THEN** identity clears with the accepted edit
- **AND** a count change clears prior capacity unless a replacement is supplied, while a curve-only change preserves it
- **WHEN** only capacity or empty voltage is changed or cleared
- **THEN** identity remains and subsequent reads preserve the override

#### Scenario: Import does not select defaults

- **WHEN** an imported profile has a known preset identity with matching S/P/profile and manually overridden or cleared capacity
- **THEN** the imported values are preserved without replaying preset defaults
- **AND** unknown identities or mismatching S/P/profile are rejected without rewriting the record

#### Scenario: Same electrical settings with a different name

- **WHEN** a selection changes only preset identity and all effective estimation values remain the same
- **THEN** the selected name persists but temporal charge tracking is not restarted
