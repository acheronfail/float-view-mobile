# Proposal

## Why

Some upstream Refloat schema ranges exceed their wire encoding capacity. The form accepts these values, then shows a generic backend-state error after write rejection, without identifying the field or a usable limit.

## What Changes

- Publish numeric wire bounds with schema fields and validate complete drafts before writing.
- Show field-level encoding-limit guidance in editor units, including limits that remain valid after editor precision rounding.
- Present generic backend error envelopes as actionable operation-specific messages; preserve stable error codes, original envelopes and useful existing messages.
- Non-goals: wrap or truncate overflowing values, change mutation admission, add retries or publish a release.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `refloat-configuration`: actionable numeric validation and write failure recovery.

## Impact

Shared codec bound calculation, schema contract/projection and mock, frontend field validation and API error presentation, canonical conformance and native boundary checks. No persistence, wire encoding or host-specific policy change. Replace raw generic API message presentation; no new dependencies.
