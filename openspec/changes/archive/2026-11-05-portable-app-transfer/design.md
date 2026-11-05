# Design

## Context

Shared Rust owns catalog/session policy and staged file workers. Hosts already present issued files; the authenticated frontend can upload binary data on every host. Individual archives contain complete session payloads and portable board identity.

## Decisions

Use an independently versioned manifest.json plus numbered sessions/\*.floatview entries in one ZIP. Bound transfers to 2 GiB, total extracted session bytes to 2 GiB, 10,000 sessions, 50 boards and 1,000 tunes; bound manifest JSON to 64 MiB. Settings carry portable auto-connect history identity, not local record IDs. Strip platform associations, private hints, active checkpoints, drafts, caches and standalone diagnostics from export. Session-embedded diagnostics follow the existing individual archive policy. Require no connected or paused ride for app operations.

Preserve the archive recording UUID as local session ID on import; validate its owned-file shape. Duplicate IDs on the same board keep the existing whole recording, including its pruning state; a conflicting board identity fails. New local board IDs remain independently issued. Preserve tune IDs across transfer. Existing matching boards/profiles and tune IDs keep destination values. A visible import-preferences option applies the incoming preferences with destination revision and safety acknowledgement preserved; auto-connect is remapped through board history identity.

Validate manifest, every exact ZIP entry, sizes, CRCs and each individual archive on a file worker. Stage validated sessions under issued session filenames absent from the catalog, keeping a bounded list and cleanup ownership; startup already reclaims unreferenced issued files. One catalog transaction attaches missing sessions/boards/tunes and optional settings. Database commit uncertainty preserves issued files for startup reconciliation. Duplicate imports discard their staged copies. Prevent mutation if the service became connected while validation ran.

Reuse the export worker and native presentation; add authenticated app-export/app-import HTTP endpoints and app.export/app.clear controls. Binary data never enters WebSocket messages. Import reports added/skipped counts. Clear uses the existing pending-delete mechanism: transactionally hide recordings, remove boards/profiles/tunes/drafts and reset preferences, then reclaim owned files; restart/retry finishes pending cleanup. Exported user files and OS permission history stay intact; diagnostics retain their separate explicit clear action.

## Risks / Trade-offs

- Existing duplicates with previously regenerated IDs cannot be inferred safely; preserve them.
- The session-artifact 128 MiB limit remains. App ZIPs use the larger bulk limit and preflight space checks; failed preparation never presents partial output.
- Transfer input plus validated session files requires a separate bounded staging reservation; use a 5 GiB total reservation ceiling and free-space admission.
- Settings import changes local preferences only and does not connect to hardware or write tunes to controllers.
