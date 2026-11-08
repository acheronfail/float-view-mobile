# Design

## Context

See proposal.md. The codec owns wire types/scales; the shared frontend currently sees only XML bounds. API exceptions retain backend messages verbatim. Existing write-timeout/unavailable feedback already preserves uncertain outcomes.

## Goals / Non-Goals

Explain invalid values before submission and remove technical generic error copy from user surfaces. Keep encoding, admission, persistence and diagnostics semantics unchanged.

## Decisions

- Calculate numeric wire bounds from the codec field type and scale, then expose nullable canonical bounds on public schema fields. Do not add persisted metadata or duplicate wire type tables in the frontend.
- Intersect editor control bounds with wire bounds. Round offered correction limits inward at editor precision, while validating unchanged canonical values against actual wire bounds. Normalize new input at editor precision using original schema bounds before wire validation.
- Show inline limit guidance under labelled fields; existing validation summary and disabled write controls provide navigation and prevent requests, including restored drafts.
- Translate only known generic backend error messages in the shared API client using operation context and stable codes. Preserve custom messages, original envelopes and code access. Use a plain-language fallback for other operations; keep existing specialized indeterminate-write handling.
- Shared hosts relay schema metadata without host-specific changes. Mock and direct conformance assert the new schema bounds; native boundaries remain covered by Rust C/JNI tests.

## Risks / Trade-offs

- A suggested decimal limit could round beyond the wire bound: round inward and test the 0.32767 / three-decimal case and signed/scaled limits.
- Error translation could hide useful detail: restrict it to exact known generic envelope text and retain the raw envelope.
- Existing unsupported values must remain inspectable: validate restored drafts without rewriting them or retrying mutations.

## Migration Plan

No stored-data or schema-record change. Bounds are derived from existing codec metadata. Existing drafts and Tunes keep canonical values and identity.
