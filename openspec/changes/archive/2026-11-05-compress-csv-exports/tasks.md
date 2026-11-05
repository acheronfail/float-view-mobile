# Tasks

## 1. Shared encoding

- [x] 1.1 Stream CSV into a DEFLATE ZIP with existing bounds and cleanup; verify product-store roundtrip, GPS and cancellation tests and scale benchmark.

## 2. Download integration

- [x] 2.1 Update HTTP metadata, frontend fallbacks and mock output; verify focused frontend tests and type checks.
- [x] 2.2 Add canonical CSV ZIP transport coverage and verify direct and mock conformance, including native-boundary service consumers.

## 3. Integration verification

- [x] 3.1 Run affected Rust tests, formatting and warnings-denied Clippy; inspect the diff and validate OpenSpec artifacts.

## Workflow follow-up

- Sync canonical specs, validate and archive after checks complete.

## Verification evidence

- Product-store CSV roundtrip verifies one DEFLATE entry, header/escaping, opt-in GPS, nearest-point ties and unmatched blanks. Both export formats pass cancellation cleanup; the uncompressed byte bound has focused coverage.
- A 100,000-sample debug benchmark exported 10,788,672 CSV bytes into a 795,154-byte ZIP in 2,398 ms. Archive export/import also passed.
- Six focused frontend tests passed; frontend Svelte/TypeScript checks reported zero errors/warnings. Changed frontend files passed ESLint/Prettier, and conformance TypeScript checking passed.
- Backend-service (136 tests plus two BLE boundary cases), loopback unit/publication tests, Apple FFI (three unit and five ABI cases), Android JNI (one case) passed. Seven loopback transport tests passed outside the sandbox after sandbox bind denial.
- Warnings-denied Clippy passed for product-store, backend-service, loopback-server and both native bindings; Rust formatting and diff whitespace checks passed.
- Direct conformance passed ZIP filename/media type, system-unzip integrity/extraction and native presentation flow; mock conformance passed. A first direct startup timed out; the rerun passed.
- Final direct and mock conformance reruns passed the compressed filename assertion. Product-store completed all 92 tests (five scale benchmarks are opt-in); the selected CSV/archive scale benchmark passed separately.
- Change validation has no issues. Canonical validation has one unchanged pre-existing long-requirement warning in “Hosts execute OS capabilities only”; that block was compared with HEAD and is unchanged.

## Matching filename follow-up

- The contained CSV now uses the ZIP filename with only its final `.zip` removed. Product-store and service presentation tests verify the names match; the mock ZIP has the same behavior.
- Focused Rust export/presentation and mock HTTP tests, frontend/conformance type checks, ESLint/Prettier, and product-store warnings-denied Clippy passed. Canonical validation retains only the unchanged warning noted above.
- Direct conformance passed, including system-unzip extraction using the CSV filename derived from the HTTP ZIP filename.
