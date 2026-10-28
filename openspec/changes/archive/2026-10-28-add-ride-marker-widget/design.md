# Design

## Context

See proposal.md. Dashboards are validated JSON in board profiles. Both native hosts use product-store; the mock uses the frontend validator. Marker creation already has a durable session-scoped API.

## Goals / Non-Goals

Keep capture discoverable and confirmation honest. Do not change session ownership, wire commands, storage baseline or native effects.

## Decisions

- Add `{kind: action, action: rideMarker}` to the widget union. A distinct action avoids treating a momentary button as a lighting switch or telemetry panel.
- Use existing activation helpers with scroll cancellation and guarded keyboard clicks. The dashboard owns hold/edit suppression and renders inert previews.
- Keep one pending request and one feedback timer per mounted button, scoped to session identity. Show success only after confirmation; ignore late results after session change or destruction. No automatic retry.
- Use orange flag/check feedback, readable text and a brief highlight. Reduced motion keeps static feedback. Existing grid dimensions remain available.
- Extend both validators together; no dependencies or native adapter changes.

## Risks / Trade-offs

- An older binary will reject the new widget variant: ship frontend and shared runtime together. Existing supported data remains valid and no marker bump is needed for this additive field variant.
- Gesture overlap could save during editing: cover hold, edit and cancelled input in browser tests.

## Migration Plan

No stored data rewrite. Existing profiles load unchanged. Add marker widgets only through explicit dashboard edits.
