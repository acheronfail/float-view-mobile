## ADDED Requirements

### Requirement: Startup presentation acceptance evidence

Startup presentation changes SHALL include executable early-theme and lifecycle regressions plus recorded visual evidence on the connected physical iOS and Android devices identified in the design. Additional device/version matrices and manual macOS visual/lifecycle testing are outside this change’s acceptance scope. Evidence SHALL distinguish native build success, frontend/backend behavior, and the actual visible handoff. The complete `just verify` gate SHALL pass before the change is accepted; failing or missing required checks SHALL remain incomplete, while excluded environments SHALL be identified as out of scope. Acceptance SHALL use the explicitly approved environment matrix in the design; out-of-scope environments SHALL NOT be reported as passed or change the supported OS range.

#### Scenario: Early theme behavior

- **WHEN** focused startup checks execute the real frontend bootstrap before application modules load
- **THEN** they verify system light/dark, explicit hints, missing/invalid/inaccessible storage, and subsequent authoritative reconciliation using rendered canvas behavior

#### Scenario: Platform visual matrix

- **WHEN** startup presentation is assessed for acceptance
- **THEN** transition recordings cover cold launch on each of the two connected devices in light and dark, including saved preferences opposite to system appearance
- **AND** evidence records the devices’ installed OS/WebView versions without requiring other devices, versions or launch behavior families
- **AND** representative launches on each connected device are repeated to detect intermittent flashes

#### Scenario: Lifecycle and failure coverage

- **WHEN** startup regression checks run
- **THEN** they retain existing automated coverage and exercise foreground/background resume, theme changes, Activity recreation, and safely reproducible failure/recovery paths on the connected devices
- **AND** native service/session continuity and bounded presentation resources remain verified through existing checks and applicable device assertions
- **AND** manual desktop checks and fault injection requiring unavailable tooling are recorded as out of scope rather than blocking acceptance

#### Scenario: Visual result is assessed

- **WHEN** a reviewer inspects a launch recording from before launch through first usable content
- **THEN** app-controlled background regions contain no unintended intermediate default color or transient loading/error screen
- **AND** intentional theme reconciliation and OS-owned animation/prompt regions are identified separately
- **AND** before/after timing observations show no added fixed startup dwell

#### Scenario: Evidence is incomplete

- **WHEN** a runtime or visual recording required by the approved matrix, or the required gate result, is missing or failing
- **THEN** the corresponding acceptance item stays incomplete with its limitation recorded
- **AND** a build result, final screenshot or unrelated successful test does not substitute for that evidence

#### Scenario: Device scope is bounded explicitly

- **WHEN** the user limits device acceptance to the currently connected iOS and Android devices
- **THEN** other devices, OS versions, simulator/emulator visual matrices and macOS manual visual/lifecycle checks are out of scope, not pending completion requirements
- **AND** existing automated checks and the complete software gate remain required
- **AND** earlier evidence is retained as supplemental without claiming verification of excluded environments
- **AND** supported backends and OS versions remain unchanged

#### Scenario: Presentation checks do not require a BLE fixture

- **WHEN** only startup presentation changes and existing software lifecycle/protocol behavior remains unchanged
- **THEN** startup acceptance, on the connected iOS and Android devices, does not require a new physical BLE checkpoint
- **AND** required phone startup observations are distinguished from the hardware-independent software gate
