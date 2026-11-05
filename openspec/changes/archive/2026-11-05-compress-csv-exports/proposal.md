# Proposal

## Why

Ride CSV exports can reach tens of megabytes while compression reduces them substantially. Compressing by default makes saving and sharing rides cheaper and faster.

## What Changes

- **BREAKING**: CSV export produces a ZIP containing one losslessly compressed CSV by default on every platform.
- Preserve telemetry rows, columns, GPS opt-in rules, and archive import/export.
- Match download filenames and media types to ZIP; retain the existing CSV request format.
- Non-goals: compression settings, raw CSV alternatives, archive compression, storage changes, or new import formats.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `shared-rust-backend-service`: default compressed CSV generation and bounded staging.

## Impact

Shared product-store encoding, loopback download metadata, frontend fallback filenames and mock coverage. Reuse the existing ZIP dependency with its Rust DEFLATE backend. Native hosts already present service filenames and support ZIP. Remove raw CSV output as the default; no storage or control-envelope changes.
