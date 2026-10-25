# Design

## Context

The shared Rust service owns one speech slot. `speech_values` already returns a valid connection generation for fresh board context even when neither reading exceeds its threshold. Native adapters play an admitted phrase until completion or an explicit cancellation.

## Goals / Non-Goals

Finish speech across threshold crossings without relaxing stale-context checks. Keep native adapters, wire contracts, storage and repeat admission unchanged.

## Decisions

Cancel active automatic speech only on invalid/mismatched context generation or its existing deadline, rather than also cancelling when both qualifying values are absent. Retain explicit rule-edit cancellation. Admission continues to require qualifying values. This reuses the existing freshness/context discriminator rather than adding a new draining state or native playback API.

Use shared-service policy tests with native speech effects/completions to verify speed, load and combined playback, cooldown, busy slots, stale context, disconnect and deadlines. Host adapters are unchanged.

## Risks / Trade-offs

- A phrase can briefly describe a value above the latest reading → only the already admitted phrase finishes; no queue is introduced and the ten-second deadline remains.
- Removing too much cancellation could retain obsolete speech → retain generation/freshness cancellation and test invalidation independently of thresholds.

## Migration Plan

No data or API migration. Ship the shared service change to both hosts; reverting the condition restores the prior behavior.
