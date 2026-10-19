## ADDED Requirements

### Requirement: Negotiated Android payload capacity

Android SHALL attempt MTU negotiation before publishing write capabilities and SHALL derive payload capacity from the confirmed MTU, subject to the shared 244-byte segmentation maximum. Rejected or failed negotiation SHALL retain default-MTU operation. Setup SHALL remain connection-qualified and bounded by the existing overall deadline.

#### Scenario: Successful negotiation

- **WHEN** Android confirms an MTU larger than 23
- **THEN** both supported write modes report the confirmed payload capacity and emitted chunks respect that capacity and the shared maximum

#### Scenario: Negotiation unavailable

- **WHEN** the negotiation request cannot start or its callback reports failure
- **THEN** service discovery continues with the default 20-byte payload

#### Scenario: Missing or stale negotiation completion

- **WHEN** negotiation never completes or an old connection completes after cancellation
- **THEN** the overall setup deadline bounds the attempt and the stale callback cannot start discovery or change replacement capabilities
