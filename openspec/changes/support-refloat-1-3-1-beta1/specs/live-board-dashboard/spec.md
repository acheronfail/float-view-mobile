## ADDED Requirements

### Requirement: Distinct live IMU timeout presentation

The live dashboard SHALL label active IMU timeout and retained IMU-timeout stop reason with accessible text independent of firmware faults and fatal-latch controls. An active timeout SHALL remain visible alongside a simultaneous firmware fault. A cleared timeout SHALL NOT appear active solely because its stop reason remains retained. Every newly introduced interactive control SHALL retain shared pointer and keyboard behavior.

#### Scenario: Active timeout

- **WHEN** the current sample reports `imuTimeoutActive` true
- **THEN** the live dashboard shows an IMU timeout error without inventing a VESC fault code or suggesting Clear fatal error as its remedy

#### Scenario: Simultaneous firmware fault

- **WHEN** an active timeout and a firmware fault coexist
- **THEN** both are identifiable and independent fatal-latch control state remains authoritative

#### Scenario: Cleared timeout and frontend reload

- **WHEN** authoritative state reports a cleared timeout with retained stop reason and the frontend is recreated
- **THEN** the view shows the stop reason without an active timeout warning and restores no stale local warning

### Requirement: Historical IMU timeout presentation

Historical review SHALL identify sampled `imuTimeout` stop transitions and recovered timeout activation/clear events distinctly from firmware faults. Recovered timing SHALL distinguish approximate occurrence from observation and SHALL NOT fabricate an occurrence time when unavailable. Existing history paging and display bounds SHALL remain enforced.

#### Scenario: Transient timeout recovered later

- **WHEN** a selected ride contains recovered timeout activation and clear between sampled observations
- **THEN** history exposes both transitions as IMU timeout events with available timing and without adding them to a firmware-fault list

#### Scenario: Unknown occurrence time

- **WHEN** source ticks cannot be placed safely on the ride timeline
- **THEN** review retains observation timing and labels occurrence timing as unavailable rather than plotting a fabricated exact instant

### Requirement: History availability follows actual faults

The Fault history action SHALL appear only for an active firmware fault or IMU timeout, a fatal latch, or actual recorded firmware/IMU history. A healthy initial inactive timeout reading SHALL NOT establish history. Cleared IMU timeouts SHALL remain accessible, including in guest mode and after frontend recreation.

#### Scenario: Healthy beta connection

- **WHEN** the beta reports inactive timeout state without faults, a latch or recorded history
- **THEN** the Fault history action is absent

#### Scenario: Cleared guest timeout

- **WHEN** an IMU timeout activates and clears during a guest connection
- **THEN** the action remains available and the bounded native history presents activation and clearance after frontend recreation

#### Scenario: Retained stop first observed after the occurrence

- **WHEN** history contains only a retained IMU-timeout stop observation
- **THEN** its occurrence time is unavailable and its observation time is shown separately

#### Scenario: Delayed recovery overlaps sampled timeout history

- **WHEN** recovered transitions arrive more than two seconds after their sampled observations
- **THEN** ordered transitions within the same observation windows replace sampled estimates once, preserving separate episodes and unknown source timing across page boundaries
