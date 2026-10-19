# Design

## Context

See proposal.md for motivation. Rust owns framing and protocol dispatch; native adapters only own GATT facts and effects. Android currently reports a fixed 20-byte capacity while Apple reports OS-negotiated capacity. The private Android capture reproduces framing failures after overlapping startup replies.

## Goals / Non-Goals

Preserve exact CRC validation, bounded storage/work, generation fencing and legitimate frame fragmentation. Keep all recovery in the shared Rust component. Do not infer telemetry identity, deduplicate payload values, change persistence, or modify the public wire contract.

## Decisions

- Keep the stateless frame scanner and exact decoder semantics. Change the stateful stream decoder to make progress after malformed candidates, retaining valid packets already decoded in the same call. Report a framing error only when no valid packets can be returned in that call; resource overflow remains an explicit error.
- Enter recovery after malformed traffic. Recovery can search beyond incomplete false headers for a complete CRC-valid frame. Normal assembly continues to trust a legitimate incomplete outer frame, avoiding accidental extraction of nested frame bytes. Bound recovery CRC work per call and preserve bounded pending input when that budget is exhausted.
- Android requests MTU 247 (244 bytes plus ATT overhead), accepts the confirmed OS result, and derives payload size from it. Rejected/failed negotiation continues at 23. A missing callback remains covered by the existing overall setup deadline. Discovery, notification setup and writes remain serialized and connection-qualified; no second reconnect policy or permanent MTU workaround is added.
- Extract only the small MTU setup state needed to unit-test successful, failed, duplicate and canceled completions without OS objects. Shared Rust continues to cap writes at 244 bytes.
- Provision temporary tools when needed. If the Play signing identity is unavailable, install a separate debug test package using a local Gradle init script; never uninstall or clear the user's Play app.

## Risks / Trade-offs

- CRC-valid embedded bytes during recovery are inherently ambiguous after corruption. Restrict searching past incomplete candidates to recovery; preserve normal fragmented outer frames and test this boundary.
- Larger MTUs previously exposed Express/Samsung corruption. Combine this change with decoder recovery and verify the connected phone; retain honest firmware-interoperability limits.
- Accepted negotiation with no callback cannot safely overlap another GATT operation. Let the existing attempt deadline cancel it rather than racing discovery.
- Exact device captures contain private data. Use synthetic public regressions and retain raw replay evidence only under ignored local storage.

## Migration Plan

No storage migration or API rollout. Validate focused Rust consumers and Android tests/build, replay the private trace, then install an isolated test copy if tools and device access permit. Rollback is source reversion; the production Play app and data remain intact.
