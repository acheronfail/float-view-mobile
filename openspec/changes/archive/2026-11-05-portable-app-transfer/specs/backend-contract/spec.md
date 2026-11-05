## ADDED Requirements

### Requirement: App transfer service contract

The shared backend SHALL provide authenticated app-export/app-import HTTP transports and app.export/app.clear controls. Binary transfers SHALL use bounded chunks and file leases; controls SHALL return bounded metadata/counts only. Apple, Android and web SHALL share validation, merge, clear and summary semantics through existing file presentation. Transfers SHALL retain a 2 GiB ceiling with bounded manifest, entry and extracted-byte counts.

#### Scenario: Untrusted ZIP content

- **WHEN** a transfer contains unknown versions, duplicate or unexpected entries, unsafe paths, oversized content or inconsistent identities
- **THEN** import fails before catalog mutation without extracting caller-selected paths

#### Scenario: Import summary

- **WHEN** an app merge succeeds
- **THEN** its result reports boards/sessions/tunes added and skipped and whether preferences were imported
