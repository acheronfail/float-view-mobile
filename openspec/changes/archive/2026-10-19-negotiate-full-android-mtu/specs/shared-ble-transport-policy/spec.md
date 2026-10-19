## MODIFIED Requirements

### Requirement: Negotiated Android payload capacity

Android SHALL request its maximum supported ATT MTU before publishing write capabilities, independently of the shared 244-byte outgoing segmentation maximum. Payload capacity SHALL derive from the confirmed MTU. Rejected or failed negotiation SHALL retain default-MTU operation. Setup SHALL remain connection-qualified and bounded by the existing overall deadline.

#### Scenario: Successful negotiation

- **WHEN** Android confirms an MTU larger than 23
- **THEN** both supported write modes report the confirmed payload capacity and emitted chunks respect that capacity and the shared maximum

#### Scenario: Notification capacity independent of write segmentation

- **WHEN** a supported peer can negotiate the maximum ATT MTU and sends notification fragments larger than the shared outgoing chunk maximum
- **THEN** the Android MTU request does not restrict negotiation to the outgoing chunk maximum plus ATT overhead and complete received fragments remain available to stream decoding

#### Scenario: Negotiation unavailable

- **WHEN** the negotiation request cannot start or its callback reports failure
- **THEN** service discovery continues with the default 20-byte payload

#### Scenario: Missing or stale negotiation completion

- **WHEN** negotiation never completes or an old connection completes after cancellation
- **THEN** the overall setup deadline bounds the attempt and the stale callback cannot start discovery or change replacement capabilities
