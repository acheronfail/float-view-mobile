# Proposal

## Why

Express can deliver overlapping startup replies followed by valid telemetry, leaving Float View's stream decoder stalled on false headers. Android also fixes transfers at 20 bytes instead of using the connection's negotiated capacity.

## What Changes

- Recover bounded, CRC-validated VESC frames after corruption without losing valid packets in the same input.
- Preserve normal fragmented frames, including payloads containing frame-like bytes.
- Negotiate Android ATT MTU before profile publication and report negotiated payload capacity to the existing shared segmentation policy.
- Remove Android's fixed 20-byte restriction; retain a default-MTU path when negotiation cannot start or fails.
- Add focused decoder/runtime and Android setup tests, replay the private capture locally, and attempt an isolated Android test installation.

Non-goals: firmware modifications, heuristic telemetry deduplication, storage/schema changes, public API changes, publishing a release, and replacing or clearing the Play installation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `shared-ble-transport-policy`: Negotiated Android transfer sizes and bounded setup fallback.
- `vesc-device-connection`: Bounded recovery of framed responses following malformed transport traffic.

## Impact

Shared Rust framing and runtime consumers on Apple/Android; Android GATT adapter and focused tests. No new production dependency, wire contract change, or persistence migration. Physical validation is scoped to the connected Android phone and Express. Build tools may be provisioned temporarily; raw captures remain ignored locally.
