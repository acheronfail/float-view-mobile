## ADDED Requirements

### Requirement: Live battery voltage warning presentation

Standard and custom battery widgets SHALL display the service-confirmed low/overVoltage warning independently of speech settings and cooldown. They SHALL show an icon, Battery low / Over voltage label, amber voltage and a full-track amber outline breathing over three seconds. High warnings SHALL add diagonal hatching. Warnings SHALL override charging animation, preserve actual fill/percentage and use no firmware fault display. Reduced motion SHALL keep steady warning cues.

#### Scenario: Low charge remains visible

- **WHEN** a low warning is confirmed at one percent charge
- **THEN** the whole track outline and Battery low label remain visible while the fill and value remain one percent

#### Scenario: High warning overrides charging

- **WHEN** an over-voltage warning is confirmed while charging
- **THEN** Over voltage, an amber outline and diagonal hatching replace the charging animation without changing the reported charge

#### Scenario: Speech is disabled

- **WHEN** battery speech is disabled, muted, unavailable or cooling down
- **THEN** a confirmed fresh warning remains visible in all battery widget sizes

#### Scenario: Recovery or invalid context

- **WHEN** the shared warning recovers, telemetry becomes stale or the connection/configuration is invalidated
- **THEN** the widget removes warning presentation according to the authoritative snapshot

#### Scenario: Reduced motion

- **WHEN** reduced motion is preferred
- **THEN** labels, steady outlines and high-voltage hatching remain with no warning animation

### Requirement: Independent visual voltage qualification

Visual battery warnings SHALL use fresh unrounded voltage from the ready selected non-BMS board independently of riding state, regen and speech eligibility. No frontend or host SHALL derive voltage thresholds. Speech thresholds, eligibility, confirmation and cooldown SHALL remain unchanged.

#### Scenario: Independent configuration availability

- **WHEN** a guest has valid Refloat limits but no cell profile, or a saved configured board has no valid Refloat limits
- **THEN** the guest can show low but cannot guess high; the configured saved board can show high without guessing low

### Requirement: Earlier low visual threshold

Low visuals SHALL use current resolved Refloat limits L < H and R = H − L, entering strictly below L + 0.25R after two continuous seconds, immediately at or below L. Low SHALL recover after two continuous seconds at or above L + 0.27R. Unavailable Refloat limits SHALL disable low independently of high.

#### Scenario: Earlier low visual without speech

- **WHEN** configured Refloat limits are 60/86 V and fresh voltage stays below 66.5 V but above 62.6 V for two seconds
- **THEN** the widget shows Battery low, including while stationary, without triggering low speech; recovery requires two continuous seconds at or above 67.02 V

### Requirement: Immediate above-full visual threshold

High visuals SHALL enter immediately strictly above the configured cell profile's zero-current full-charge voltage multiplied by its validated series count, independently of capped estimated percentage. High SHALL recover after two continuous seconds at or below 99.5% of that voltage and take priority over low. Missing, invalid or mismatched estimation configuration SHALL disable high independently of low.

#### Scenario: Immediate above-full visual

- **WHEN** a valid configured profile has a full-charge pack voltage of 84 V and the first fresh measurement is strictly above 84 V while stationary or drawing power
- **THEN** the widget immediately shows its high warning even if the estimated percentage is capped at 100, without triggering speech solely because of that visual state

#### Scenario: Equality and recovery

- **WHEN** voltage equals the full-charge voltage with no active warning, or an active high warning falls briefly below its recovery boundary
- **THEN** equality does not enter high and brief recovery does not clear it; sustained fresh recovery for two seconds clears it

### Requirement: Visual freshness and context isolation

Visual inputs SHALL be less than two seconds old. Gaps of at least two seconds and changed threshold, controller or connection context SHALL reset qualification. Profile edits that change or invalidate visual thresholds SHALL clear prior qualification and require subsequent fresh samples.

#### Scenario: Gap or profile edit

- **WHEN** observations stop for at least two seconds or the configured profile changes
- **THEN** prior warning qualification clears and the next sample cannot bridge confirmation or recovery across the gap or edit
