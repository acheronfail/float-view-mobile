# Design

## Context

The preview owner supplies static presentation facts and explicit profile-save callbacks. Live intercepts taps to configure; Custom starts a draft automatically. Pure widget previews suppress controls through the same flag used by the board preview. See proposal.md for motivation.

## Decisions

- Keep the preview boundary explicit. The board preview owner owns one bounded pair of example light states and passes interaction callbacks to the two renderers. It never routes a simulated action through the backend or fabricates a writable active connection.
- Reuse normal tap/hold gestures and the existing persisted appearance toggle. Only the interactive board surface receives example interactions; picker/configuration thumbnails remain inert. Marker widgets retain their unavailable presentation without a session.
- Keep Live distance switching local to its displayed box until an explicit configuration selection changes the saved field. Reset transient overrides when the saved slot changes or the preview is left.
- Remove forced Custom draft entry and preview-specific Save/Cancel navigation. Reuse normal viewing, hold-to-edit, drafts and leave guards. The header Back action remains the route exit.

## Risks / Trade-offs

- Interactive controls could escape to a connected board → explicit preview callbacks plus browser assertions prohibiting device/marker requests while another board is connected.
- Reusing theme could activate during a hold or thumbnail → retain existing gesture ownership and thumbnail suppression; cover pointer, keyboard and cancellation paths.
- Preview light states could leak across targets → own them in the keyed board-preview component; test reset after leaving/reopening.

## Migration Plan

No storage or public-contract changes. Existing layouts and saved default-view preferences remain valid; no new dependency or native work.
