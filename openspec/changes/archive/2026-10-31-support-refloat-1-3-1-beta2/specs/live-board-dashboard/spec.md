# Spec Delta

## ADDED Requirements

### Requirement: IMU timeout warning and historical presentation

The live dashboard SHALL identify an active IMU timeout as a Refloat error without calling it a controller firmware fault or fatal latch. The existing history experience SHALL present timeout activation and clearing beside firmware faults under inclusive wording. A retained IMU stop reason SHALL remain visible as a stop reason after the active error clears.

#### Scenario: Concurrent alerts

- **WHEN** an IMU timeout and controller firmware fault are both active
- **THEN** the live warning and accessible status disclose both without hiding either or offering fatal clear solely for the timeout

#### Scenario: Cleared timeout history

- **WHEN** command 35 recovers a timeout activation and clearing during a saved ride or guest connection
- **THEN** the existing history list shows both with distinct labels and approximate or unavailable ride times, and remains discoverable after clearance

#### Scenario: Healthy connection

- **WHEN** beta2 reports inactive timeout with no fault, latch, or actual alert history
- **THEN** the history action is not shown solely because the timeout capability exists

#### Scenario: Retained stop and recreation

- **WHEN** a cleared timeout leaves stop condition `imuTimeout`, or the frontend is recreated after history was saved
- **THEN** the stop label and recorded history recover from authoritative state without asserting a currently active timeout
