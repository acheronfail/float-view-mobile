# Spec Delta

## ADDED Requirements

### Requirement: Dashboard appearance toggle

Live box choices SHALL offer Light / dark mode. Activating it SHALL persist the opposite effective theme as an explicit light or dark preference through existing revision-checked settings. System appearance SHALL remain selectable in Settings. Pending or unavailable writes SHALL block duplicate activation; failed writes SHALL retain authoritative appearance and show an error.

#### Scenario: Toggle from system or explicit appearance

- **WHEN** the rider activates the toggle by primary tap or keyboard/assistive click
- **THEN** exactly one settings update selects the opposite currently effective theme and survives frontend recreation

#### Scenario: Configure without changing appearance

- **WHEN** the rider selects the box content, holds to configure, scrolls, cancels a pointer or uses a non-primary pointer
- **THEN** no appearance update occurs
