# Proposal

## Why

Saved-board dashboards currently require a connection to edit, although their configuration is stored locally. Riders should be able to arrange both views with a clear board identity and useful previews before riding.

## What Changes

- Add Customise views to saved-board history and connected saved-board menus.
- Show the selected board's name and configured artwork with Live View and Custom View choices.
- Reuse direct Live slot selection and Custom draft editing with labelled, fixed example readings and no runtime actions.
- Preserve the selected board through connection changes, existing revisions and independent layouts.

## Capabilities

### Modified Capabilities

- `app-navigation`: board-scoped view customisation entry points independent of connection.
- `custom-board-dashboard`: direct offline editing, illustrative previews and target isolation.

## Impact

Shared Svelte frontend, profile helpers and browser coverage. Reuse existing profile operations and stored fields on all hosts; no wire, native or storage-format change. Remove active-connection dependencies from reusable dashboard persistence and presentation paths. Non-goals: firmware settings, simulated connections, new dashboard types, changing next-connected view by preview browsing, commits and online changes.
