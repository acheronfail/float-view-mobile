# Proposal

## Why

Requesting ATT MTU 247 exposes an Express notification-fragmentation defect: a 416-byte schema reply loses three bytes at its first fragment boundary. The reply arrives promptly, so increasing configuration timeouts cannot repair it.

## What Changes

- Request Android's maximum supported ATT MTU while retaining the existing shared outgoing segmentation cap and confirmed-capacity fallback.
- Add a regression that models Express's notification fragmentation and detects the three-byte truncation at smaller negotiated sizes.
- Verify complete configuration bootstrap, live telemetry and reconnects on the connected Android device; retain captures privately.
- Replace the restricted MTU request; add no dependency or compatibility framework.

Non-goals: firmware changes, longer timeouts, lighting-priority changes, public API or storage changes, board configuration writes, and replacing the Play installation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `shared-ble-transport-policy`: Negotiate maximum supported Android ATT capacity independently of the outgoing segmentation cap.

## Impact

Android MTU negotiation, JVM regression coverage and transport documentation. Apple and shared runtime behavior remain unchanged. No persistence migration, dependency or wire-contract change. Physical validation covers the connected Android/Express combination; iOS MTU remains unmeasured.
