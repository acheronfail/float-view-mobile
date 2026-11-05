## Purpose

Define portable app backups, repeatable non-destructive merging and explicitly requested clearing of saved product data across supported platforms.

## Requirements

### Requirement: Versioned portable app export

Settings SHALL offer one app-data export containing an independently versioned manifest, all saved current board profiles, saved tunes, portable preferences and completed session archives. Export SHALL preserve recording/board identities and exclude OS associations, controller UUIDs, checkpoints, drafts, caches and standalone diagnostics. The shared service SHALL require the app to be disconnected with no paused ride and SHALL use bounded native presentation or browser download.

#### Scenario: Empty and populated boards

- **WHEN** app data is exported
- **THEN** the transfer includes every saved board, including boards without recordings, using its current profile rather than a historical session snapshot

#### Scenario: Export progress and cancellation

- **WHEN** the user exports app data
- **THEN** the shared export modal shows preparation and completed-session counts and supports cancellation before native presentation

### Requirement: Repeatable app merge import

Settings SHALL offer one app-data import with an explicit preferences-import choice. Import SHALL validate the complete package before catalog mutation, add missing boards, sessions and tunes, preserve destination profiles/tunes on matching identities, and report added/skipped counts. Import SHALL preserve destination safety acknowledgement and remap auto-connect through portable board identity. Import SHALL neither replace the whole dataset nor connect to or configure hardware.

#### Scenario: Repeated and round-trip imports

- **WHEN** the same transfer or its imported/re-exported equivalent is imported again
- **THEN** recordings and tunes are skipped by stable identity, existing board/profile records remain intact, and no duplicates are added

#### Scenario: Import in progress

- **WHEN** a confirmed import is being validated and merged
- **THEN** a blocking modal shows an indeterminate status until completion and the resulting summary reports added and skipped records

#### Scenario: Malformed package or interruption

- **WHEN** manifest, entries, archive validation or capacity checks fail before commit
- **THEN** destination product data is unchanged and issued temporary files are reclaimed
- **AND** interruption around catalog commit preserves files referenced by a committed catalog and reclaims unreferenced issued files on restart

### Requirement: Explicit saved-data clearing

Settings SHALL offer a separate destructive confirmation for clearing saved boards, recordings, profiles, tunes, drafts and app preferences. Clearing SHALL require a disconnected app without a paused ride or active file/maintenance workers, preserve OS permissions and exported user files, and leave diagnostics under its existing separate clear action. Logical deletion SHALL commit atomically; interrupted reclamation SHALL remain restartable.

#### Scenario: Clear and re-import

- **WHEN** the user confirms clearing and then imports a valid app transfer
- **THEN** the cleared dataset is restored through the normal merge importer without a replace-from-backup mode
