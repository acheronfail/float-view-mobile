# Application Appearance Specification

## Purpose

Define the global appearance preference and consistent frontend theme resolution across native hosts and disposable WebViews.

## Requirements

### Requirement: Persisted appearance preference

Global settings SHALL contain a `themePreference` with exactly `system`, `light`, or `dark` as valid values. Its default SHALL be `system`, and updates SHALL use the same revision-checked, native-owned persistence semantics as other settings.

#### Scenario: Default appearance

- **WHEN** no valid persisted appearance preference exists
- **THEN** authoritative settings expose `themePreference` as `system`

#### Scenario: Explicit appearance update

- **WHEN** the frontend updates the preference to `light` or `dark` at the current settings revision
- **THEN** subsequent authoritative state and WebView recreation preserve that explicit preference

#### Scenario: Invalid appearance update

- **WHEN** a settings patch contains an appearance preference outside the three valid values
- **THEN** the backend rejects the patch and leaves persisted settings unchanged

### Requirement: Effective theme resolution

The frontend SHALL resolve `light` and `dark` preferences directly and SHALL resolve `system` from the host's current `prefers-color-scheme` result. It SHALL apply the effective theme to application surfaces, controls, browser color-scheme integration, and host theme color without platform-specific branches.

#### Scenario: Explicit theme

- **WHEN** the authoritative preference is `light` or `dark`
- **THEN** the frontend uses that theme regardless of the current system preference

#### Scenario: System theme

- **WHEN** the authoritative preference is `system`
- **THEN** the frontend uses the system's current light or dark preference

#### Scenario: System preference changes

- **WHEN** the system color preference changes while `system` is selected
- **THEN** the effective application theme updates without a backend settings mutation

#### Scenario: Explicit preference ignores system changes

- **WHEN** the system color preference changes while `light` or `dark` is selected
- **THEN** the effective application theme remains unchanged

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
