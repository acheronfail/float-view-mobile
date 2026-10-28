# Proposal

## Why

Flywheel mode reports wheel motion and released footpads without a rider. Automatic ride announcements currently mistake this for riding and speak misleading warnings.

## What Changes

- Suppress automatic footpad, speed, load and battery speech while flywheel is authoritative, including cancellation of an admitted automatic phrase.
- Reset transient speech qualification on entry and require fresh non-flywheel observations on exit, preserving cooldowns and preferences.
- Exclude recorded flywheel intervals from both session footpad-off-at-speed overlays while preserving raw recorded events and samples.
- Keep raw telemetry, recording, electrical/fault evidence, manual speech previews and existing flywheel-disable behavior.
- Non-goals: changing ride statistics, GPS eligibility, firmware safety behavior, device mutation policy or the flywheel overlay.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ride-announcements`: Flywheel exclusion and transition handling for automatic speech.
- `session-location-review`: Flywheel exclusion for derived footpad fault markers.

## Impact

Shared Rust backend announcement policy and shared frontend session footpad detection with focused tests; applies to Apple and Android through the existing owners. No public API, persistence format, dependencies or compatibility changes. Existing recordings receive corrected derived markers when reviewed. No superseded paths to remove.
