# Design

## Context

See proposal.md. iOS embeds one WKWebView; Svelte owns routed return destinations. `floatview:back` already dismisses the top surface or navigates to the current return destination. WebKit browser-history gestures would not match that policy.

## Decisions

- Install a left UIScreenEdgePanGestureRecognizer in the iOS WebView coordinator. Gate beginning on rightward horizontal intent and completion on distance or sufficient forward velocity with minimum travel; cancelled/failed/reversed gestures do nothing.
- Publish only a boolean nested-back capability from the shared shell to an optional iOS script-message handler. Validate the handler name, boolean body, main frame and current backend origin. The host knows no route names. Default disabled, reset on document replacement/process failure, and re-publish on frontend mount/presentation changes.
- Send the existing cancelable `floatview:back` event on successful release. Keep existing modal/editor guards and scroll restoration. No backend request, persisted state, native route stack or new dependencies.
- Require the document scroll recognizer to wait for the edge recognizer to fail; reject vertical intent promptly so ordinary scrolling remains available. Allow simultaneous recognition with WebKit content-touch recognizers so they cannot suppress the native edge recognizer; the explicit scroll-pan failure rule still applies. Native recognition cancels web content touches for accepted edge drags. Browser history swipes remain disabled.

## Risks / Trade-offs

- Gesture feel and chart/map arbitration need targeted checking → native UI coverage for edge vs non-edge/short/vertical paths, plus user device trial.
- No finger-following transition → completion navigates through existing route motion; interactive transitions remain a later decision.
- Host/frontend lifecycle races → closed capability defaults off and is reset on replacement; shared frontend rechecks eligibility when dispatching a completed gesture.

## Migration Plan

No data migration. Reverting removes only ephemeral presentation integration.
