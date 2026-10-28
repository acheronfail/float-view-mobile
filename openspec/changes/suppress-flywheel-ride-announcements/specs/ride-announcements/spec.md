## ADDED Requirements

### Requirement: Flywheel automatic speech exclusion

While authoritative flywheel mode is active, the service SHALL suppress automatic footpad, speed, load and battery charge speech and cancel an admitted automatic phrase. It SHALL reset transient speech qualification without changing saved settings or process-local cooldowns. Telemetry, raw recording, visual battery evidence, fault evidence and manual speech previews SHALL retain their existing behavior.

#### Scenario: Enter flywheel during automatic speech

- **WHEN** authoritative telemetry enters flywheel while an automatic phrase is active
- **THEN** that phrase is cancelled and no automatic phrase is admitted, even with released footpads, high wheel speed, high load or qualifying battery voltage

#### Scenario: Flywheel timers and unavailable mode

- **WHEN** flywheel remains active across telemetry updates, timer callbacks, missing or unknown mode observations, or WebView replacement
- **THEN** automatic speech stays suppressed and no alert qualification time accumulates

#### Scenario: Resume after flywheel

- **WHEN** fresh telemetry confirms a known non-flywheel mode after flywheel
- **THEN** automatic rules evaluate fresh evidence with a full new footpad/battery qualification delay where applicable and retain prior routine and battery cooldowns, without replaying suppressed alerts

#### Scenario: Reconnect or controller replacement

- **WHEN** a connection is replaced or restored after flywheel
- **THEN** previous speech and qualification cannot trigger an alert and the replacement requires fresh eligible telemetry under existing connection isolation rules

#### Scenario: Manual previews and visual evidence

- **WHEN** flywheel is active and a manual speech preview is requested or fresh electrical/fault telemetry arrives
- **THEN** the preview retains existing bounded admission/audio behavior and telemetry, recording and visual warning evidence remain available under their existing eligibility rules
