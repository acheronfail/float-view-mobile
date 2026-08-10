## 1. Shared Apple FFI support

- [x] 1.1 Add the dependency-free `apple-ffi-support` workspace crate with documented protocol-neutral boundary errors and no exported C symbols.
- [x] 1.2 Implement panic containment, checked regions, alignment/disjointness validation, validated input borrowing, and bounded byte/scalar publication with explicit unsafe contracts.
- [x] 1.3 Add exhaustive unit tests for valid, zero-length, null, misaligned, wrapping, overlapping, insufficient/exact-capacity, publication, and panic cases.
- [x] 1.4 Add workspace audits that enforce the support dependency boundary, absence of external dependencies/C symbols, and exclusive ownership of shared raw-memory primitives.

## 2. Apple bridge migration

- [x] 2.1 Migrate firmware identity, BMS values, and controller data Apple bridges to shared support while preserving their exact decode statuses, records, capacities, and adversarial ABI behavior.
- [x] 2.2 Migrate VESC framing, custom configuration, and Refloat control Apple bridges to shared support while preserving every encode/decode/scan/control symbol and result shape.
- [x] 2.3 Delete superseded bridge-local containment, region, alias, slice, copy, and scalar-publication implementations plus duplicate primitive-only tests.
- [x] 2.4 Verify all six bridge manifests depend only on their domain crate and `apple-ffi-support`, with no Android JNI crate or host framework dependency.

## 3. Build integration and documentation

- [x] 3.1 Add the support manifest/source to all six iOS Xcode Rust phase inputs and audit exact incremental invalidation coverage.
- [x] 3.2 Update shared build/catalog and repository audits for the current six bridge families and twelve artifacts without changing any package, archive, library, or symbol identity.
- [x] 3.3 Update core and Apple documentation to distinguish shared boundary mechanics from independent domain bridges and document why generation/macros remain out of scope.

## 4. Software verification

- [x] 4.1 Pass formatting plus warnings-denied focused support/workspace tests and linting.
- [x] 4.2 Pass all six focused Apple C ABI/adversarial suites and exact symbol/dependency/raw-memory ownership audits.
- [x] 4.3 Pass clean macOS, iOS device, and iOS simulator Debug/Release bridge/package builds and unchanged Android host/four-ABI Debug/Release packaging.
- [x] 4.4 Pass strict OpenSpec validation and the complete `just verify` software gate with no generated or committed native artifacts.

## 5. Physical regression and final gate

- [x] 5.1 Run identity, direct/CAN control, BMS, and background scenarios as applicable on the connected Android device against the ESP32 fixture.
- [x] 5.2 Run the same applicable identity, direct/CAN control, BMS, and background scenarios on the connected iOS device against the ESP32 fixture.
- [x] 5.3 Run identity, direct/CAN control, and BMS scenarios on the macOS host against the ESP32 fixture.
- [x] 5.4 Confirm cleanup and every non-location assertion on all hosts; defer only bounded-time location sample absence.
- [x] 5.5 Rerun structural audits and the complete `just verify` gate after physical evidence succeeds.
