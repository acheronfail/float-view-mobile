## ADDED Requirements

### Requirement: Actionable numeric encoding limits

Configuration schemas SHALL expose canonical numeric encoding bounds. The editor SHALL reject values beyond those bounds before writing, identify each affected field and show a usable limit in editor units and precision. Input controls SHALL respect encoding limits as well as schema bounds. Invalid restored drafts MUST remain visible and correctable without board traffic. Backend validation MUST remain authoritative.

#### Scenario: Unsigned integer overflow

- **WHEN** a field allows 100000 in its schema but its encoding stores at most 65535
- **THEN** the editor identifies that field, explains the controller limit of 65535 and blocks writing until corrected

#### Scenario: Decimal limit and editor precision

- **WHEN** a field stores at most 0.32767 and its editor uses three decimal places
- **THEN** overflow feedback offers 0.327 or lower as a usable correction and never recommends a value that rounds to an overflowing encoding

#### Scenario: Restored overflow

- **WHEN** an existing draft contains an overflowing value
- **THEN** opening the editor retains the value, displays field-specific guidance and disables writing without sending a mutation

### Requirement: Actionable configuration error presentation

Configuration operations SHALL present generic backend error responses as understandable recovery guidance for the attempted action, preserving stable error codes and original error envelopes for diagnostics. Useful specific messages SHALL remain intact. Write failures MUST retain edits, avoid automatic retries and preserve uncertainty when a write may have reached the controller.

#### Scenario: Generic state rejection

- **WHEN** an eligible-looking configuration write receives a generic invalid-state error
- **THEN** the user sees guidance to check connection and board readiness and reread current values, without backend-state terminology

#### Scenario: Specific rejection

- **WHEN** the backend supplies a specific actionable rejection
- **THEN** that explanation remains visible rather than being replaced by generic recovery copy

#### Scenario: Indeterminate write

- **WHEN** write confirmation times out or the controller becomes unavailable
- **THEN** feedback preserves the possibility that the write reached the controller and asks for a fresh current read before retrying
