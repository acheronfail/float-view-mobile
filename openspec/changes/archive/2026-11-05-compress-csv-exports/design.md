# Design

## Context

The shared product store streams CSV into a leased stage. The service derives presentation names from that stage, while the loopback route supplies media types. Android already recognises ZIP; Apple presenters use service filenames. See proposal.md for motivation.

## Goals / Non-Goals

Stream compressed output through existing workers and transfer leases. Do not change request enums, archive formats, storage, import policy or host ownership.

## Decisions

Use the existing zip 6 dependency with its flate2 Rust backend and DEFLATE level 6. ZIP has familiar extraction support and existing host presentation support; gzip would be less convenient for users opening files. Reuse flate2 already present in the workspace instead of implementing an archive container.

Write one safe entry named after the ZIP with its final .zip removed, directly through ZipWriter over the buffered stage file. Preserve the current uncompressed CSV byte counter, final compressed size check, cancellation checks and failure cleanup. Finish and flush the ZIP before renaming to .csv.zip or publishing it. No intermediate plain CSV or ride-sized memory buffer.

Keep the CSV action name and request format, update the browser fallback filename and accepted media type. Mock downloads use a small valid compressed ZIP. Canonical transport checks verify ZIP metadata and extracted CSV across backends.

## Risks / Trade-offs

- Users must extract the CSV before opening it in tools that require plain text. This is the intentional output-format change.
- Compression adds CPU work to the existing bounded background export worker; a representative scale check measures size and latency.
- Compression ratio depends on telemetry; tests verify lossless extraction and reduction on representative repetitive data without promising a fixed ratio.

## Migration Plan

No stored-data migration. Existing archives and previously exported CSV files remain valid. Reverting the encoder restores raw output for future exports.
