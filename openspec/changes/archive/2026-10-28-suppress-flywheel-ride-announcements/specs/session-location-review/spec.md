## ADDED Requirements

### Requirement: Flywheel exclusion from session footpad fault overlays

Session review SHALL omit both one-footpad and both-footpads-off-at-speed markers during recorded flywheel intervals. Recorded mode transitions SHALL break footpad episode continuity without rewriting stored samples or events. Known non-flywheel mode SHALL end exclusion; absent or unknown mode SHALL not clear established exclusion. Sessions without mode history SHALL retain their existing behavior.

#### Scenario: Recorded flywheel motion

- **WHEN** retained speed samples exceed the footpad overlay threshold while recorded package mode is flywheel
- **THEN** neither footpad overlay produces a marker regardless of recorded footpad contact

#### Scenario: Mode and footpad history across pages

- **WHEN** mode and footpad transitions or exact speed samples span read pages, including transitions sharing a timestamp
- **THEN** exclusion uses elapsed time and source sample sequence, never a future transition, and pagination does not duplicate an episode

#### Scenario: Exit and sparse retained samples

- **WHEN** a known non-flywheel transition follows flywheel, including when no speed sample remains inside that interval
- **THEN** the next qualifying non-flywheel speed sample starts a new footpad episode without extending the pre-flywheel episode

#### Scenario: Missing mode history

- **WHEN** a recording contains no package-mode transitions or contains unknown mode after an established flywheel transition
- **THEN** no-history recordings retain ordinary detection, while established flywheel exclusion persists until known non-flywheel mode is recorded

#### Scenario: Existing recordings

- **WHEN** a saved or imported recording is reopened with the footpad overlays enabled
- **THEN** derived markers apply the same exclusion while raw telemetry, mode events, firmware faults, ride statistics and GPS records remain unchanged
