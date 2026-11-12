# Proposal

## Why

Riders need a quick way to switch appearance without leaving their dashboard.

## What Changes

- Add a light/dark toggle box to Live and an optional widget to Custom.
- Toggle the effective theme to the opposite explicit preference through existing settings persistence.
- Extend the dashboard action vocabulary with `themeToggle`, preserving existing dashboards.
- Non-goals: changing system-theme settings, native theme policy, or dashboard defaults.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-appearance`: dashboard access to the existing persisted appearance preference.
- `custom-board-dashboard`: selectable and persistable theme action widget.

## Impact

Shared Svelte widget, Live layout, custom picker/configuration, TypeScript contract and Rust dashboard validation. Apple and Android pass dashboard JSON through the existing shared service; no new host operation or storage migration. Extend canonical conformance and persistence coverage. No new dependencies or superseded paths.
