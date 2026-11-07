# Design

## Context

Diagnostic records currently permit only closed codes/counts/flags. Incident persistence assumes three slots and marker 1; older collectors reject unfamiliar files. Existing event, incident and report limits remain applicable. The user explicitly authorizes previously excluded technical content but retains location, credential and device-identity exclusions.

## Goals / Non-Goals

Capture useful configuration contents, actual schema hashes/signatures, non-location telemetry and classified protocol payloads without enabling arbitrary dumps. Keep native boundaries, public envelopes, product storage and polling unchanged. CAN topology collection and automatic upload remain outside scope.

## Decisions

- Raise the incident count to ten. Keep the 8 MiB journal/incident budget by retaining a shared 3 MiB incident budget, two 2 MiB journal segments and at most a 1 MiB incident staging file. Evict oldest eligible incidents for either count or bytes, in memory and on disk. Keep seven-day expiry, capture windows and export limits.
- Add bounded optional content fragments to diagnostic records. Old records omit them. Use explicit categories, offsets and total byte lengths; preserve per-event bounds and disclose truncation. Actual schema hashes pass strict lowercase SHA-256 validation. Requested hashes are retained only when recognized by the audited catalogue; an arbitrary client string is not safe merely because it resembles a digest. Keep observed schema identity separate from audited write identity so rejected schemas remain diagnosable without granting capabilities. Configuration contents come from parsed schema fields and known configuration input fields, never arbitrary request text. Nonempty configuration text is redacted and raw packets containing such text are omitted to preserve credential/path/identity exclusions.
- Classify protocol payloads in the coordinator alongside scheduler facts. Strip CAN wrappers. Capture safe requests and validated fixed numeric/configuration responses. Skip identity, unknown and location-bearing raw families with closed reasons. Retain decoded non-location telemetry from existing normalized samples, excluding extension names and absolute observation times. Diagnostic data never changes routing or protocol decisions.
- Configuration contents are captured at user operation boundaries and value changes so failures retain the applicable board/draft state. Numeric telemetry uses a bounded diagnostic cadence; wire capture follows existing request records and does not change device polling.
- Use diagnostic marker/report version 2. Validate all v1 contents before atomically replacing its marker using a bounded staging file and directory sync. Preserve old row contents and old slot placement. Select existing/free/oldest incident slots rather than changing legacy modulo placement. Read v1/v2 rows explicitly; reject unknown versions. No generic migration framework or product marker change.

## Risks / Trade-offs

Richer data can fill existing event budgets sooner; maintain all numeric bounds, clipping/loss disclosure and byte-budget eviction tests. Sanitizing unknown bytes is unreliable; skip those packets instead of assuming they contain no location or identities. Configuration JSON and packet fragments need reassembly, documented in the report. Local collection is still manually exported, and privacy copy must describe included technical content accurately.

## Migration Plan

Test v1 reopen, interrupted marker staging, unknown/newer markers and corruption without destructive recovery. The atomic marker transition permits safe restart. Older app versions preserve an unsupported v2 directory and fail diagnostics actionably; they must not rewrite it. Existing catalogs, recordings and archives remain untouched.
