## ADDED Requirements

### Requirement: Equivalent native flywheel mutation lifecycle

Apple and Android SHALL retain hand-written but behaviorally equivalent application-service ordering for current-generation Refloat telemetry, mutation confirmation, flywheel-mode cancellation, configuration invalidation, state publication, waiter completion, and bounded timeout. Platform Bluetooth adapters and WebView hosts SHALL remain outside this coordination rule and MUST NOT implement a parallel confirmation path.

#### Scenario: Equivalent enable lifecycle

- **WHEN** either native host receives flywheel-mode telemetry for its pending enable action
- **THEN** its application service confirms through the mutation coordinator and returns one successful complete state

#### Scenario: Equivalent disable lifecycle

- **WHEN** either native host receives repeated flywheel telemetry followed by normal-mode telemetry for its pending disable action
- **THEN** it preserves then confirms the action with the same externally observable state ordering

#### Scenario: Disposable WebView

- **WHEN** the WebView is absent or recreated during either transition
- **THEN** native confirmation, timeout, and safety cleanup continue independently and a later state snapshot exposes the authoritative result

