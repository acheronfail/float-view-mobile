## MODIFIED Requirements

### Requirement: Shared product persistence preserves semantics

The shared service SHALL own settings, saved boards, profiles, private routing associations, schema caches and Tunes. It SHALL preserve canonical validation/defaults, optimistic revisions, identity compatibility, bounded pagination, cache integrity and eviction, immutable session snapshots and separation of private local identifiers from portable board identity. Invalid updates MUST fail atomically with the existing error semantics.

#### Scenario: Conflicting settings or profile update

- **WHEN** two clients update the same product record using its old revision
- **THEN** only the first valid update commits and the other receives the canonical conflict without lost fields or extra revision advancement

#### Scenario: Save and apply a Tune

- **WHEN** a Tune is created, changed or applied
- **THEN** shared code validates its exact package/schema/wire identity, complete values and current revisions before persistence or any board mutation

#### Scenario: Corrupt schema cache

- **WHEN** cached data has the wrong identity, digest or size
- **THEN** the service rejects that cache entry and follows the existing bounded discovery behavior without treating it as authoritative configuration

#### Scenario: Portable board association

- **WHEN** an archive introduces a known board identity without a local Bluetooth association
- **THEN** its history and profile are available while connecting still requires a valid locally discovered association

#### Scenario: Replace incompatible development storage before first release

- **WHEN** the shared service encounters a recognised pre-baseline development marker and owned layout
- **THEN** it resets only validated owned development files to baseline 100 once, uses the current identity algorithm without development readers or aliases, and preserves supported or unsupported data on future launches; corruption and I/O failure do not authorize deletion
