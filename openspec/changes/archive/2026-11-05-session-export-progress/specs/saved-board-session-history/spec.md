## ADDED Requirements

### Requirement: Bulk export progress dialog

Board export SHALL open an accessible modal immediately and block repeated activation. It SHALL show preparation status followed by completed and total session counts, offer Cancel during preparation, and remain open while accepted cancellation stops the worker. Success, cancellation and failure SHALL be distinct. Dismissal during preparation SHALL request cancellation, and native save/share cancellation SHALL remain available after preparation.

#### Scenario: Cancel preparation

- **WHEN** the user cancels a board export while its worker is running
- **THEN** the dialog reports cancellation in progress until cleanup finishes, no partial file is downloaded or presented, and another export can subsequently start

#### Scenario: Large session encoding

- **WHEN** one session takes a long time to encode
- **THEN** the dialog retains honest completed-session counts and does not fabricate within-session percentages
