# Design

## Context

See proposal.md. The shared coordinator currently discards rejected INFO identities, and its unsupported_refloat issue also covers missing/malformed INFO. INFO v2 has a fixed 20-byte version suffix. Both hosts serialize the same shared server projection.

## Goals / Non-Goals

Keep admission separate from display metadata and preserve native ownership. Do not add a second firmware catalog, host-specific UI, a flashing path, or persistence.

## Decisions

- Add a bounded INFO identity decoder retaining the source-defined suffix; reject invalid UTF-8/control bytes. Existing admission rejects nonempty suffixes, keeping prereleases unsupported. Keep only display version metadata for failure, never raw INFO, hashes or private identity.
- Add optional `ConnectionIssue.refloatCompatibility` containing `reportedVersion` (at most 32 ASCII bytes) and `supportedVersions` (at most 32 exact catalog releases). Shared projection derives the list from the compatibility catalog. Only confirmed unsupported releases carry this field. Existing consumers without the field keep generic failure behavior.
- Use existing identification_timeout for missing INFO and a closed refloat_info_invalid issue for unreadable INFO. Unsupported layout retains its existing generic issue without version-specific metadata.
- Prefer failed runtime state over reconnecting presentation; terminate pending recovery after discovery failure rather than allowing transport loss to re-enter recovery.
- Use the existing dialog with reversed action handlers for primary Back to boards and secondary Check again, plus a native details disclosure. Backdrop/Escape follow the primary exit. Preserve focus handling and busy/error guards. Reset the dialog key each attempt so an identical subsequent rejection reopens it.
- No new dependencies. Replace only the confirmed incompatibility's generic dialog and diagnostics hint.

## Risks / Trade-offs

- Malformed display text → bounded ASCII suffix decoding; do not echo arbitrary bytes.
- Catalog drift → backend derives source releases from its existing catalog.
- Retries overlap or stale events → existing coordinator generations and frontend busy guard remain authoritative; test repeated failure and recreation.
- Native BLE test availability → use the discoverable ExpressT only for identification/recovery; never flash or issue motor/configuration writes. Keep device-specific evidence ignored locally.
