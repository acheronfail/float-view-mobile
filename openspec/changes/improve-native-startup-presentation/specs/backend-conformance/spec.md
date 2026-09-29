## ADDED Requirements

### Requirement: Startup presentation acceptance evidence

Startup presentation changes SHALL include executable early-theme and lifecycle regressions plus recorded visual evidence for iOS, macOS and Android. Evidence SHALL distinguish native build success, frontend/backend behavior, and the actual visible handoff. The complete `just verify` gate SHALL pass before the change is accepted; unavailable environments or failing checks SHALL remain explicitly unverified.

#### Scenario: Early theme behavior

- **WHEN** focused startup checks execute the real frontend bootstrap before application modules load
- **THEN** they verify system light/dark, explicit hints, missing/invalid/inaccessible storage, and subsequent authoritative reconciliation using rendered canvas behavior

#### Scenario: Platform visual matrix

- **WHEN** startup presentation is assessed for acceptance
- **THEN** transition recordings cover cold launch on each supported platform in light and dark, including saved preferences opposite to system appearance
- **AND** evidence includes both mobile launch behavior families and minimum/current supported OS coverage identified in the design
- **AND** current-platform representative launches are repeated to detect intermittent flashes

#### Scenario: Lifecycle and failure coverage

- **WHEN** startup regression checks run
- **THEN** they cover retained and disposed frontend resume, activity/window recreation, existing renderer recovery, startup failure and recovery, delayed web loading, and backend unavailability as applicable to each host
- **AND** native service/session continuity and bounded presentation resources remain verified

#### Scenario: Visual result is assessed

- **WHEN** a reviewer inspects a launch recording from before launch through first usable content
- **THEN** app-controlled background regions contain no unintended intermediate default color or transient loading/error screen
- **AND** intentional theme reconciliation and OS-owned animation/prompt regions are identified separately
- **AND** before/after timing observations show no added fixed startup dwell

#### Scenario: Evidence is incomplete

- **WHEN** a platform runtime, visual recording, or required gate result is missing or failing
- **THEN** the corresponding acceptance item stays incomplete with its limitation recorded
- **AND** a build result, final screenshot or unrelated successful test does not substitute for that evidence

#### Scenario: Physical hardware is not part of the change

- **WHEN** only startup presentation changes and existing software lifecycle/protocol behavior remains unchanged
- **THEN** simulator/emulator and desktop acceptance does not require a new physical BLE checkpoint
- **AND** optional real-device visual observations are recorded separately from required software evidence
