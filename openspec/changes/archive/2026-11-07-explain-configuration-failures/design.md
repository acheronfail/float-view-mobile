# Design

## Context

Runtime errors are currently discarded by the service before API failure recording. Configuration, mutation and scheduler owners already expose bounded state, but it is absent from incidents. API failures are preserved after the follow-up window; associated causal context must survive too.

## Goals / Non-Goals

Explain rejected and accepted-but-failed operations using existing owners. Do not change authorization, caller-facing errors, contracts, native interfaces or product storage. Do not add raw errors or configuration data.

## Decisions

Instrument shared runtime and service operation boundaries. Capture structural admission facts before applying a command and emit them on failure. Use explicit enum matches for runtime categories and existing authoritative configuration blockers. Compare requested identity and revision in memory and retain only flags and revision counts. Record pending request correlation and outcomes, plus detailed asynchronous configuration/mutation failures. Preserve related failure events through bounded incident coalescing. Keep successful logging limited to user operations rather than polling.

Use the existing diagnostic serializer and retention limits. New vocabulary is additive; historical files remain readable and no migration is required. Both hosts inherit the change through the existing service. Recognized stable/beta release suffixes remain closed compatibility facts rather than arbitrary firmware text. Exported operation namespaces are independently rebased, and the report explains that occurrence counts represent incident triggers.

## Risks / Trade-offs

More events consume the existing event budget; keep each record below 1,024 bytes and test saturation. A snapshot is evidence of state at a boundary, not proof of an upstream firmware defect. Test actual rejection and export paths, including late coalescing, sanitization and unchanged public errors. Run affected Rust suites, service consumers/native boundary checks and focused conformance where relevant.
