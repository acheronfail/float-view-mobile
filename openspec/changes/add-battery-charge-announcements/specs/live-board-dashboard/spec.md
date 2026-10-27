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

- **WHEN** the shared warning recovers, riding stops, telemetry becomes stale or the connection/configuration is invalidated
- **THEN** the widget removes warning presentation according to the authoritative snapshot

#### Scenario: Reduced motion

- **WHEN** reduced motion is preferred
- **THEN** labels, steady outlines and high-voltage hatching remain with no warning animation
