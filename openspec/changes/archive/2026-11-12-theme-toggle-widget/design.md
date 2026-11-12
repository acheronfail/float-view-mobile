# Design

## Context

See proposal.md. Live slots accept a closed field vocabulary; custom actions already support ride markers. Hosts pass profile and settings JSON into the shared Rust owner.

## Goals / Non-Goals

Reuse existing appearance persistence and dashboard interactions. No new host operation, default widget, dependency or migration.

## Decisions

Add `themeToggle` to Live fields and custom actions, excluding it from numeric readings. Extend shared TypeScript/mock and Rust validation together. A shared rune controller resolves system preference reactively and guards settings writes; Live uses its existing hold handler and Custom a reusable button. This preserves configurable Live slots instead of adding a fixed row. Preview disables writes. Existing settings errors remain authoritative.

## Risks / Trade-offs

- Synthesized clicks or editing could toggle twice → reuse guarded activation and dashboard gesture suppression, with browser tests.
- System changes could stale the next target → subscribe to media changes and resolve from current settings.
- Older app binaries reject the new optional vocabulary → additive acceptance in this version; existing configurations and baseline fixtures remain intact.

## Migration Plan

No data rewrite. Existing dashboards load unchanged. Apple and Android continue passing opaque profile JSON to shared validation; canonical conformance exercises their existing operations.
