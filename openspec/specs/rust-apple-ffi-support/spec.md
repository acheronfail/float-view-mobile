# Apple Coarse C Boundary Safety

## Purpose

Define the C memory and failure contract of the retained application-runtime Apple binding; the former per-protocol support crate is retired.

Original cutover requirements and evidence remain in [archived changes](../../changes/archive/).
Ongoing checks follow the [verification policy](../../../AGENTS.md#verification).
The capability identifier remains stable for historical references.

## Requirements

### Requirement: Coarse Apple C boundary safety

The selected Apple binding SHALL validate bounded call-local regions before borrowing or publishing, contain panics and use fixed host-runtime statuses. It SHALL retain no caller pointer, invoke no callback and expose no pointer or private payload in diagnostics. Unsafe memory handling SHALL remain at this coarse boundary, not in portable domain owners.

#### Scenario: Validate caller memory

- **WHEN** a C call receives byte regions and scalar outputs under its documented caller memory obligations
- **THEN** it checks required non-null pointers, nonzero-length regions, scalar alignment, checked address extents, bounds and pairwise disjointness before access
- **AND** a null byte pointer is accepted only where the operation permits a zero-length region; invalid regions fail without constructing a slice or publishing payload bytes

#### Scenario: Apply and copy a service result

- **WHEN** a valid command is applied and its stored result is copied into caller-owned memory
- **THEN** the shared host envelope supplies the exact ABI version, fixed status and complete result bytes
- **AND** invalid or insufficient output capacity publishes no partial payload; retrieving a result never applies the command again

#### Scenario: Contain a panic

- **WHEN** a boundary operation panics
- **THEN** no unwind crosses the C ABI and the caller receives the fixed internal-failure status

#### Scenario: Audit retained boundary ownership

- **WHEN** source, symbols and affected ABI/adversarial tests are checked
- **THEN** only the coarse Apple binding is selected, its unsafe operations are explicit, and no independent protocol ABI, generic support crate or generated native artifact is required
