# Proposal

## Why

Settings has become a long stream of unrelated controls. Users need predictable destinations and focused pages to find and change preferences without scrolling through everything.

## What Changes

- Replace the controls on Settings home with seven labelled navigation rows and short previews.
- Group existing controls into Appearance & units, Ride announcements, Ride recording, Connection & permissions, Data & backups, Help & diagnostics and About.
- Put recording technical controls behind Advanced recording; group announcement options by alert and retain Off / On segmented controls for boolean preferences.
- Preserve existing saves, native actions and bounded navigation restoration; remove the unrelated ride-total subtitle and superseded flat layout.
- Non-goals: new preferences, changed defaults, audio policy, storage formats, backend or native contract changes, search.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-navigation`: Focused settings destinations and progressive disclosure.

## Impact

Shared Svelte frontend, existing navigation/restoration and affected browser tests. No dependency, public API, persistence or platform policy changes. Subpages restore to Settings home after frontend recreation under the existing checkpoint contract.
