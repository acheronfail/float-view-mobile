## MODIFIED Requirements

### Requirement: Stable initial appearance

Before ordinary application content is presented, the frontend SHALL apply the best locally available effective theme and SHALL reconcile it with authoritative settings after the backend connects. Local presentation state MUST NOT override a different authoritative preference.

#### Scenario: First launch

- **WHEN** no local appearance hint or persisted setting is available
- **THEN** the first rendered theme follows the system preference

#### Scenario: Recreated frontend

- **WHEN** a disposable WebView is recreated with a locally cached appearance hint
- **THEN** the frontend applies the hint for initial presentation and then reconciles to the native-owned setting

#### Scenario: Stale local hint

- **WHEN** the local hint differs from the setting returned by `state.get`
- **THEN** the frontend applies the authoritative preference and refreshes the hint

#### Scenario: Appearance hint is unavailable or invalid

- **WHEN** local appearance storage is inaccessible, empty, or contains an invalid preference
- **THEN** early frontend presentation follows system appearance and application startup continues

#### Scenario: Application assets are delayed

- **WHEN** application styles or modules have not yet loaded
- **THEN** the document canvas already uses the best locally available effective theme without waiting for hydration or backend connectivity

## ADDED Requirements

### Requirement: Continuous startup appearance

Across iOS, macOS and Android, app-controlled launch surfaces, native backgrounds and the initial web canvas SHALL use the application palette for the best theme available at that stage. The transition SHALL NOT introduce an intermediate default-colored surface or transient loading message. OS-owned launch presentation before application preferences are available SHALL follow system appearance; a later deliberate transition to a saved or authoritative preference SHALL be distinguished from an unintended flash.

#### Scenario: Matching system and application appearance

- **WHEN** the application launches with system appearance or a matching explicit preference
- **THEN** its launch, native and web backgrounds remain visually continuous until application content is rendered

#### Scenario: Saved preference differs from system appearance

- **WHEN** the OS launches the application in one appearance and the saved app preference is the opposite
- **THEN** application surfaces adopt that preference when it becomes available without oscillating through unrelated default colors

#### Scenario: Return to system appearance

- **WHEN** the user changes an explicit app preference back to system after the OS appearance has changed
- **THEN** both native and web presentation follow the current OS appearance, including after window or frontend recreation
