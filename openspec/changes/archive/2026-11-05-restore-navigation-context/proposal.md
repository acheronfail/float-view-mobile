# Proposal

## Why

Returning from a session loses the board list's loaded pages and scroll position. Users browsing older rides must repeatedly scroll and reload pages.

## What Changes

- Retain bounded, frontend-lifetime browsing positions independently for routed pages and boards.
- Restore visible list anchors, loaded session pages and keyboard focus on return, including header, native and browser back and tab switching.
- Reconcile changed catalogs without reusing stale cursors and fall back near deleted anchors.
- Add deep pagination and navigation browser coverage.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-navigation`: retain browsing context across routed navigation.

## Impact

Shared frontend shell, session list and browser tests. No new dependencies, backend contracts, native host changes or durable persistence. Existing navigation activation remains shared. No superseded public paths to remove. Non-goals: retaining chart inspection state, surviving frontend recreation, or changing overlay navigation semantics.
