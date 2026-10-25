# Design

## Context

The telemetry owner already holds resumable ride time. Session summaries persist the greatest absolute sampled speed in m/s. Live and Custom share a closed reading catalog, validated by both TypeScript and the shared Rust store.

## Goals / Non-Goals

Keep one bounded peak in the existing ride owner and restore through the current time restore path. Do not introduce view-owned tracking, polling firmware statistics, storage changes, or host-specific calculations.

## Decisions

Add optional maxSpeedKph to live telemetry and maxSpeedKph to the widget catalog. Update the maximum using absolute finite observed speed in the existing normalized sample path. Preserve across reconnect, clear on new recording, and restore from the existing session maxSpeedMps after conversion. Forward the field through shared server projection used by Apple and Android. No historical sample field is added.

Reuse existing numeric formatting and picker/persistence flows, including custom range domains and imperial conversions. The mock mirrors the ride peak and resume behavior. Firmware statistics were rejected because board boot/reset lifetime differs from the current ride; frontend accumulation would lose state on WebView disposal. No dependency is needed.

## Risks / Trade-offs

- Offline peaks cannot be observed → keep recording gaps honest.
- Process replacement may lose uncommitted samples → restore the existing committed summary, consistent with saved ride summaries.
- New reading identifiers must pass both validators → test saved Live slots and Custom ranges through canonical conformance.

## Migration Plan

Additive contract and catalog change shipped through the shared backend for both hosts. Existing supported summaries seed restoration; no data migration or archive version change. Rollback affects presentation only.
