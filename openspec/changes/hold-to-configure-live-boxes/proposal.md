# Proposal

## Why

Current ride consumes scarce dashboard space and tap-to-configure prevents boxes from acting as controls. Move charts to the menu and use holding for configuration so lighting can be operated directly from configurable boxes.

## What Changes

- Keep five slots; add Empty, Headlights and LEDs as selectable box types.
- Hold a box to configure it; tap lighting boxes to request confirmed runtime changes.
- Default to Empty, Wattage, Session timer, Odometer, then Trip distance in the centre.
- Replace Live's Current ride button and menu lighting items with Charts…; retain the Sessions entry.
- Non-goals: extra dashboard slots, new lighting commands, telemetry/storage migrations, resetting user-selected boxes or changing guest history policy.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `live-board-dashboard`: hold configuration, lighting boxes, defaults and action-menu placement.
- `app-navigation`: charts entry in the live menu.

## Impact

Frontend contract catalogue, shared Rust settings validation/defaults used by Apple and Android, client/mock and canonical conformance. Existing five-string persistence shape and lighting API remain unchanged; no dependencies or compatibility machinery added. Remove superseded lighting menu generation and Live-only Current ride placement.
