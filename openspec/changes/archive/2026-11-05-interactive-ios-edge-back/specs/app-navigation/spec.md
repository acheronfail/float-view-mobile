## MODIFIED Requirements

### Requirement: Native iOS edge back gesture

On iOS, a deliberate completed rightward swipe from the left screen edge SHALL invoke the shared Back action when a nested back destination is available. It SHALL preserve dismissal/save guards and retained browsing context. Primary destinations without a nested back control, short or reversed drags, vertical scrolling and cancelled gestures SHALL NOT navigate. The current page SHALL track eligible drags, settle back on cancellation, and slide away after accepted Back and parent restoration.

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

#### Scenario: Interactive feedback and guarded completion

- **WHEN** an eligible edge drag begins and moves
- **THEN** the current viewport follows the finger over a neutral back affordance without reflowing its charts or fixed controls
- **AND** release completes the slide only after accepted parent navigation and restored presentation; cancellation, failed navigation or surface dismissal settles the moving layer back and removes it

#### Scenario: Interrupted presentation

- **WHEN** the app backgrounds, the document changes, or snapshot capture fails during a gesture
- **THEN** temporary presentation is bounded and cleaned up, without stale callbacks moving a new document or replaying Back

#### Scenario: Reduced motion

- **WHEN** Reduce Motion is enabled
- **THEN** direct finger tracking remains available and automatic settling uses reduced motion
