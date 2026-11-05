## ADDED Requirements

### Requirement: Native iOS edge back gesture

On iOS, a deliberate completed rightward swipe from the left screen edge SHALL invoke the shared Back action when a nested back destination is available. It SHALL preserve dismissal/save guards and retained browsing context. Primary destinations without a nested back control, short or reversed drags, vertical scrolling and cancelled gestures SHALL NOT navigate. Navigation SHALL occur after release without an interactive page-drag transition.

#### Scenario: Nested return

- **WHEN** a user completes an eligible edge swipe from session detail, board history or another nested page
- **THEN** the app performs the same return as its existing Back handler, including list-position restoration

#### Scenario: Guarded surface

- **WHEN** an eligible edge swipe completes while a dismissible surface or guarded editor is above a nested page
- **THEN** shared dismissal and save guards handle the request before any parent navigation

#### Scenario: Ineligible gesture

- **WHEN** the gesture is short, reversed, cancelled, primarily vertical or starts away from the left edge
- **THEN** no Back action is dispatched

#### Scenario: Primary destination or frontend reconstruction

- **WHEN** no nested back destination is available, or a new frontend has not published its presentation readiness
- **THEN** the native edge gesture remains disabled without using browser history or stale presentation state
