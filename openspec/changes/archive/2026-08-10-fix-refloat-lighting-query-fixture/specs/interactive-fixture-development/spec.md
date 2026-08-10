## ADDED Requirements

### Requirement: Read-only lighting refresh during flywheel simulation

The interactive and canonical fixture coordinator SHALL use the same source-faithful lighting model. While simulated flywheel mode is active it SHALL answer exact zero-mask legacy and stable lighting status queries from current disposable state, SHALL preserve that state, and SHALL continue to reject every nonzero-mask lighting mutation until eligible flywheel disable returns the package to normal mode.

#### Scenario: Interactive flywheel lighting refresh

- **WHEN** a connected native backend performs its bounded zero-mask lighting refresh while the interactive fixture is in flywheel mode
- **THEN** the coordinator returns current LED and headlight state without recording a mutation or changing either value

#### Scenario: Interactive flywheel lighting mutation

- **WHEN** the same peer sends a nonzero-mask LED or headlight mutation while flywheel mode remains active
- **THEN** the coordinator supplies no successful response and preserves all simulated state

