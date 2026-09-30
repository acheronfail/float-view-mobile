## ADDED Requirements

### Requirement: Mandatory permission inventory and recovery

Every supported backend SHALL expose a bounded permission inventory in complete state snapshots and implement authenticated `permissions.recover` with a closed permission identifier and expected action. The result SHALL report current permission state, not assert a grant merely because presentation or settings navigation succeeded. Existing Bluetooth and location permission methods SHALL use consistent underlying state and recovery policy. The frontend SHALL render the advertised inventory and actions without platform detection or optional-method negotiation.

#### Scenario: Current action
- **WHEN** an authenticated client requests an applicable advertised recovery action
- **THEN** the backend dispatches native presentation and returns one terminal result or structured error, with subsequent authorization observations published in complete monotonic snapshots

#### Scenario: Invalid or stale target
- **WHEN** a request contains an unknown identifier or action, an inapplicable permission, or an action that no longer matches current state
- **THEN** malformed identifiers/actions receive invalid_request and inapplicable or stale actions receive invalid_state without native presentation

#### Scenario: Presentation cannot proceed
- **WHEN** another permission presentation is pending or no foreground host can present
- **THEN** the backend returns conflict or unavailable respectively without accumulating deferred prompts

#### Scenario: Stale completion
- **WHEN** a timed-out or prior-lifetime native operation completes
- **THEN** it does not complete a newer request or overwrite newer state, and current OS observations remain authoritative

#### Scenario: Authentication and bounds
- **WHEN** a permission command has invalid authentication, origin, or exceeds contract bounds
- **THEN** it is rejected before any native effect using the existing transport and error rules
