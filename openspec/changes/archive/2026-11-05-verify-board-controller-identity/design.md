# Design

## Context

The shared service passes the selected controller UUID to ProductStore before creating/attaching a saved session. ProductStore already derives a deterministic portable identity and stores a nullable unique platform transport association independently of profiles and history.

## Goals / Non-Goals

Make verified UUIDs authoritative with the existing schema. Preserve all supported records. No transfer format, history-merging UI, aliases, firmware changes or platform-specific identity policy.

## Decisions

Validate UUID before mutation. In one immediate catalog transaction, look up the UUID-derived identity, clear the requested local transport association from any other saved board, then update the exact match or create a separate board with its default profile. Capacity or database failures roll back detachment too. Old board/profile/session records are never renamed, merged or deleted. A different UUID creates a different board even when its transport is familiar.

Remove random-identity creation/local fallback from saved activation; existing random/imported opaque identities remain supported records and stay offline until an exact verified identity can match. Adapt synthetic test setup to supply meaningful identities, preserving the frozen baseline fixtures.

Use existing shared-service protocol replay to verify recording/profile ownership and snapshot publication after a controller changes behind the same transport. No host changes or BLE hardware timing changes are required.

## Risks / Trade-offs

- Previously mixed history cannot be reliably separated: preserve it unchanged; the rule controls future associations and recordings.
- A shared CAN transport has one local association: retain the existing last-selected hint and reverify UUID on activation rather than adding an association table.
- Board capacity can prevent creation of a replacement-controller record: fail without changing the old association or any history.
