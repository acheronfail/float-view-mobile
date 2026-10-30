# Design

## Context

See proposal.md for motivation. Live boxes and Custom widgets share `LiveWidgetField` and display helpers. Apple and Android both persist settings/profile selections through the Rust product store.

## Goals / Non-Goals

Provide identical derived values in both views without adding telemetry or persistence columns. Leave existing counter semantics and compatible presentations unchanged.

## Decisions

- Add `wattHoursNetWh` and `ampHoursNetAh` to the shared selectable catalogue and Rust allowlist. Compute display values from the current sample, as with battery power; transmitting duplicate derived telemetry would add unnecessary contract surface.
- Require both operands to be finite. Missing readings remain unavailable rather than becoming zero. Preserve negative results for net regeneration.
- Use Wh with one decimal and Ah with two; positive consumption uses draw colour, negative uses charging colour, zero is neutral. Keep number and trend presentations, matching existing accumulated counters.
- Native adapters require no edits because they delegate this contract to the same Rust implementation. Mock validation already consumes the TypeScript catalogue.

## Risks / Trade-offs

- Catalogue drift between frontend and Rust → exercise both identifiers in canonical mock/direct conformance and persistence tests.
- A derived total cannot compensate for device counter resets → retain source-counter behavior rather than invent ride offsets.

## Migration Plan

This adds accepted identifiers to existing settings JSON without changing supported stored data, baseline markers or archive formats. Ship the frontend and shared runtime together.
