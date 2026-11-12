# Spec Delta

## ADDED Requirements

### Requirement: Theme toggle widget

Custom widget selection and configuration SHALL offer Light / dark mode with supported dimensions. Its identity, order and dimensions SHALL persist through the shared Apple and Android profile contract without changing existing dashboards. It SHALL use the same persisted appearance toggle as Live, independently of telemetry availability.

#### Scenario: Save and restore the widget

- **WHEN** a rider adds or resizes the theme widget, saves and reloads
- **THEN** the widget configuration is preserved and activation toggles appearance once

#### Scenario: Preview and edit suppression

- **WHEN** the widget is previewed, held to edit, dragged, scrolled, cancelled or activated by a non-primary pointer
- **THEN** appearance remains unchanged
