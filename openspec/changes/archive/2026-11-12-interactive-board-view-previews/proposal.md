# Proposal

## Why

Offline previews currently use different tap behavior from riding views and open Custom directly in editing. Riders should be able to try their layout's ordinary interactions before holding to edit it.

## What Changes

- Use normal Live tap/hold behavior and open Custom in viewing mode.
- Allow the ordinary persisted app theme toggle in interactive previews.
- Share temporary frontend-only headlight/LED examples across both views; keep device commands and ride recording isolated.
- Return Custom to viewing after Save or Cancel without leaving the preview.

## Capabilities

### Modified Capabilities

- `custom-board-dashboard`: representative preview interactions, edit entry, theme behavior and isolated example controls.

## Impact

Shared frontend renderers, preview owner and focused tests/specs. Reuse existing gesture and appearance owners; remove preview-only tap-to-edit and forced-edit branches. No native, wire, storage-format or dependency change. No simulated board commands, fake marker capture, commits or online changes.
