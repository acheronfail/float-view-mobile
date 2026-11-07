# Tasks

## 1. Retention and storage

- [x] 1.1 Retain ten incidents within existing byte bounds in memory and on disk; test rollover, reopen, expiry and clear.
- [x] 1.2 Upgrade v1 diagnostic storage explicitly and restartably; test preserved rows, interrupted staging, unknown markers and corruption.

## 2. Authorized diagnostic content

- [x] 2.1 Add bounded configuration/telemetry fragments and actual schema identities; test serialization, truncation, value inclusion and retained exclusions.
- [x] 2.2 Capture classified protocol bytes through the coordinator; test direct/CAN routing removal, safe bytes and identity/GNSS/unknown exclusions.
- [x] 2.3 Update report format, omissions and user-facing privacy copy; test ZIP reassembly, historical records and export bounds.

## 3. Integration

- [x] 3.1 Run focused Rust/consumer/native checks, frontend checks for privacy copy, diagnostics conformance, formatting and Clippy; record evidence.
- [x] 3.2 Validate and sync canonical diagnostics requirements against the implementation.

## Workflow follow-up

- Archive after implementation and verification complete.
