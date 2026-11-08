## ADDED Requirements

### Requirement: Source-correct numeric editor presentation

Numeric configuration fields SHALL use the active schema's VESC Tool editor units, decimal precision and increment steps. Percentage editors SHALL use whole percentages of the declared maximum absolute bound. Edited inputs SHALL normalize at editor precision before conversion to canonical firmware units. Untouched values, stored Tunes, draft values, schema fingerprints and wire signatures MUST retain their meaning. Existing overflow rejection and mutation eligibility SHALL remain enforced.

#### Scenario: Scaled percentage field

- **WHEN** brightness has canonical value 0.5, editor scale 100 and step 5
- **THEN** the editor and review display 50 percent, accept 50 as canonical 0.5 and increment by five displayed percentage points

#### Scenario: Editor decimal precision

- **WHEN** a user enters 18.19 for a one-decimal current field
- **THEN** the draft uses canonical 18.2 and the settled editor and confirmation display 18.2

#### Scenario: Percentage editor

- **WHEN** the duty tiltback field uses upstream percentage editing
- **THEN** its editor displays whole percentages and converts them to the corresponding canonical fraction

#### Scenario: Restored canonical draft

- **WHEN** an existing saved draft or Tune contains canonical fractions and numeric input text
- **THEN** the editor converts their presentation without reinterpreting stored fractions as percentages or changing untouched values

#### Scenario: Invalid or incomplete input

- **WHEN** numeric input is incomplete, non-finite or outside the declared bounds
- **THEN** the editor retains actionable validation and does not enable a write using a previously valid value
