# Proposal

## Why

Flywheel mode reports wheel motion and released footpads without a rider. Automatic ride announcements currently mistake this for riding and speak misleading warnings.

## What Changes

- Suppress automatic footpad, speed, load and battery speech while flywheel is authoritative, including cancellation of an admitted automatic phrase.
- Reset transient speech qualification on entry and require fresh non-flywheel observations on exit, preserving cooldowns and preferences.
- Keep raw telemetry, recording, electrical/fault evidence, manual speech previews and existing flywheel-disable behavior.
- Non-goals: changing ride statistics, GPS eligibility, firmware safety behavior, device mutation policy or the flywheel overlay.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ride-announcements`: Flywheel exclusion and transition handling for automatic speech.

## Impact

Shared Rust backend announcement policy and focused tests; applies to Apple and Android through the existing owner. No public API, persistence format, dependencies or compatibility changes. No superseded paths to remove.
