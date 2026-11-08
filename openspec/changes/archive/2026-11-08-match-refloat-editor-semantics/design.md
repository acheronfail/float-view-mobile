# Design

## Context

See proposal.md. Compressed source XML owns identity and is persisted in the schema cache; normalized schema records are transient codec interchange. Draft numeric text and typed values currently use canonical firmware units.

## Goals / Non-Goals

Use upstream numeric presentation on the shared frontend without modifying supported storage, wire encoding or mutation policy. Do not force untouched board or Tune values through editor precision.

## Decisions

- Add finite positive editor scale and percentage-mode metadata to normalized fields and the public schema contract. Normalize absent metadata to VESC Tool defaults (scale 1, percentage off). Fingerprints remain hashes of the original XML.
- Extend transient schema-record field flags with optional editor metadata; absent flags retain the existing defaults. Value records and persisted source XML are unchanged. This is an additive record extension, not a storage upgrade.
- Keep canonical units in controller state, persisted input text, requests and Tunes. The field component owns transient displayed text while editing, converts numeric input to canonical units at editor precision, and settles text on blur. Existing saved input text is rendered through the same unit conversion.
- Match the vendored mobile VESC Tool editor's Math.round precision normalization, including negative half steps. Wire rounding remains the separately audited VESC serializer behavior. Percentage editors use scale 100/max(abs(bounds)), precision 0 and step 1.
- Use the same presentation conversion in confirmation differences. Keep invalid input visible; bounds validation still blocks submission rather than silently clipping values.
- Hosts already relay the shared schema response; verify the shared native boundaries instead of adding host-specific schema logic. The hand-written mock publishes the same editor metadata; direct and mock conformance must assert it.

## Risks / Trade-offs

- Raw vs displayed units can corrupt a draft if converted twice: test canonical persisted restoration and exact outgoing values.
- Rounding while typing can disrupt edits: retain transient display text and settle on blur while canonical draft values are already normalized.
- Upstream desktop and mobile editors can differ at negative half steps: use the explicit vendored mobile editor algorithm; test signed ties.
- Unknown metadata must fail closed; validate bounds on scale and precision without altering existing identities.

## Migration Plan

No durable format change. Read existing canonical draft text unchanged and transform only its display. Existing source-schema caches are reparsed. Optional record flags preserve prior records with default editor metadata. Reverting presentation code preserves stored canonical values.
