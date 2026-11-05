## MODIFIED Requirements

### Requirement: Shared archive and export behavior

The shared service SHALL own archive validation/import, manifest generation, CSV export, staging and cleanup. It SHALL preserve archive formats, board identity, session metadata, GPS export policy, CSV columns and matching rules. Native code SHALL only acquire authorized external file access or present service-issued files. File bytes MUST remain bounded streams rather than control envelopes, and client input MUST NOT authorize arbitrary filesystem paths.

#### Scenario: Import a large valid archive

- **WHEN** an authorized archive within the existing 128 MiB bound exceeds the control envelope size
- **THEN** bounded streaming validates and attaches it with the existing local-session and portable-board identity semantics without allocating an archive-sized buffer

#### Scenario: Reject hostile archive input

- **WHEN** an archive has malformed SQLite/schema/metadata, a path escape, or a stale or foreign transfer handle
- **THEN** validation fails without altering existing product records or accessing files outside the authorized root and partial staging is reclaimed

#### Scenario: Export while frontend disconnects

- **WHEN** an OS save/share presentation still owns a generated export and the frontend disconnects
- **THEN** the file remains valid for that presentation and is reclaimed after completion or bounded abandonment without retaining unlimited staging

#### Scenario: Preserve location export policy

- **WHEN** a completed session is exported with GPS export disabled or enabled
- **THEN** shared encoding preserves the existing privacy policy, column shape and nearest-location matching behavior on every host

## ADDED Requirements

### Requirement: Compressed CSV exports

CSV export SHALL produce a ZIP containing exactly one DEFLATE-compressed CSV by default on every platform. The CSV SHALL preserve existing telemetry content and GPS opt-in behavior. Download and save/share filenames SHALL end in `.csv.zip`; HTTP downloads SHALL use `application/zip`. Export SHALL retain existing byte bounds, cancellation and staging cleanup without buffering the complete CSV.

#### Scenario: Save or share a ride CSV

- **WHEN** a completed ride is exported as CSV through a browser or native host
- **THEN** the result is a ZIP with one safely named CSV whose name matches the ZIP filename with only the final `.zip` removed and whose extracted rows and columns match the existing CSV encoding

#### Scenario: Cancel or exceed the export limit

- **WHEN** CSV generation is cancelled or exceeds the existing uncompressed or final file byte bound
- **THEN** export fails and partial staging is reclaimed without presenting a successful file
