## ADDED Requirements

### Requirement: Persisted global unit preference

Global settings SHALL contain `unitPreference` with exactly `metric` or `imperial` as valid values. Its default SHALL be `metric`, and updates SHALL use the same revision-checked, native-owned persistence semantics as other settings.

#### Scenario: Default units

- **WHEN** no valid persisted unit preference exists
- **THEN** authoritative settings expose `unitPreference` as `metric`

#### Scenario: Imperial update

- **WHEN** the frontend updates the preference to `imperial` at the current settings revision
- **THEN** subsequent authoritative state and WebView recreation preserve that preference

#### Scenario: Invalid unit update

- **WHEN** a settings patch contains a unit preference outside the two valid values
- **THEN** the backend rejects the patch and leaves persisted settings unchanged

### Requirement: Consistent measurement presentation

The frontend SHALL apply the global unit preference consistently to live and session measurements without changing canonical backend, persistence, series, diagnostic, or export values. Metric presentation SHALL use kilometres per hour, kilometres or metres as appropriate, and Celsius. Imperial presentation SHALL convert those values to miles per hour, miles or feet as appropriate, and Fahrenheit. Voltage, current, duty percentage, sample counts, time, and eRPM SHALL remain unit-system independent.

#### Scenario: Metric live and session views

- **WHEN** `unitPreference` is `metric`
- **THEN** live and session speed, distance, and temperature render with the corresponding metric values and labels

#### Scenario: Imperial live and session views

- **WHEN** `unitPreference` is `imperial`
- **THEN** live and session speed, distance, and temperature render with consistently converted imperial values and labels

#### Scenario: Preference changes

- **WHEN** the authoritative unit preference changes while a measurement view is rendered
- **THEN** displayed values and labels update without mutating or refetching the underlying measurements

#### Scenario: Export under imperial presentation

- **WHEN** the user exports a session while imperial presentation is selected
- **THEN** native export semantics and canonical stored values remain unchanged
