# Design

## Context

At negotiated ATT MTU 247 the physical capture contains two copies of each schema reply, both missing exactly bytes 244–246 of the expected 416-byte frame. The next notification starts at byte 247. Replies begin within 78–143 ms. The vendor Express Bluetooth implementation uses `min(mtu, 255)` rather than subtracting the three-byte ATT overhead. Linux downloaded the complete audited schema through the same board under a different transport setup.

## Goals / Non-Goals

Restore complete schema and motor-limit reads on Android while preserving confirmed OS capacity, bounded setup and shared segmentation. Do not change firmware, decoder CRC rules, scheduling priorities, request deadlines, storage or public contracts.

## Decisions

Request ATT MTU 517, the Android-supported maximum. Express can then send its capped 255-byte notifications within ATT capacity when the peer confirms a sufficiently large MTU. Continue deriving advertised write capacity from the actual callback, and retain the shared 244-byte outgoing cap. Negotiation failure, cancellation and missing callbacks keep their existing behavior.

Test the request policy against the observed peer fragmentation model: slice synthetic response bytes at `min(mtu, 255)` and model notification truncation at `mtu - 3`. The prior 247 request must lose bytes; the selected request must preserve every byte. Keep private wire evidence local.

## Risks / Trade-offs

A peer can negotiate less than requested; report the confirmed value rather than assuming 517. This change cannot repair firmware truncation at every possible smaller negotiated MTU. Physical acceptance on the affected phone/board must establish the negotiated size and complete bootstrap. iOS success alone does not prove its negotiated MTU.

## Migration Plan

No stored-data or API change. Install over the isolated BLE test application, preserving its data and the original Play installation. Rollback is the previous test APK.

## Open Questions

Confirm repeated cold downloads and cached reconnects on the affected Android device. Original default-MTU overlapping traffic remains covered by decoder recovery; maximum-MTU configuration reliability needs its own physical evidence.
