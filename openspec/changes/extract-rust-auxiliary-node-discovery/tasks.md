## 1. Candidate Model and Bounds

- [x] 1.1 Add `float-view-auxiliary-node-discovery` to the locked Rust workspace with unsafe code forbidden and only safe controller discovery as a production dependency.
- [x] 1.2 Define bounded statuses, candidates, generations, stable issues, generic owned observation state, and ordered host effects with private diagnostics redacted.
- [x] 1.3 Implement exact stock-VBMS/VBMS32 admission, checked routes, finite CAN normalization, exact 64-device/16-candidate limits, display-safe projections, and total invalid-input outcomes.

## 2. Auxiliary Discovery State Machine

- [x] 2.1 Implement begin, direct BMS selection, preferred route inventory reuse/direct probing, complete-inventory automatic selection, and incomplete-inventory CAN scan effects.
- [x] 2.2 Implement sequential firmware probes, probe and scan timeout, unique/not-found/ambiguous/unavailable outcomes, stable failures, and no controller-readiness coupling.
- [x] 2.3 Implement generation-qualified observation attachment, stale snapshot retention, unavailability/recovery, reset, repeated-event behavior, and wrapping generation invalidation without executing host work.

## 3. Parity and Adversarial Evidence

- [x] 3.1 Add a committed shared auxiliary-discovery TSV and replay every source-derived native direct, preferred, reusable-inventory, enumeration, selection, availability, and reset case in Rust.
- [x] 3.2 Cover exact supported and unsupported BMS tuples, mixed routed inventories, direct precedence, missing responses, sorted ambiguity, and canonical seventeenth-BMS overflow behavior.
- [x] 3.3 Exhaust every CAN ID and 64/65 plus 16/17 boundaries; permute duplicates, stale/mismatched results, timeouts, observations, failures, resets, counter rollover, repeated transitions, and private diagnostic values.

## 4. Isolation and Repository Integration

- [x] 4.1 Add focused root test and audit recipes and wire the candidate into workspace verification and the root audit allowlist.
- [x] 4.2 Audit the exact safe dependency graph, absence of FFI/JNI/callback/runtime/storage/transport code, unchanged bridge catalogues, absence from Apple/Android package inputs, and retained native auxiliary-discovery sources.
- [x] 4.3 Document the candidate's direct Rust composition boundary, unchanged native owners, canonical overflow decision, eventual bridge/native deletion path, and recommended next unified-runtime slice.

## 5. Verification

- [x] 5.1 Run focused tests and audits, formatting, warnings-denied locked workspace linting, strict OpenSpec validation, and `git diff --check`.
- [x] 5.2 Run `just verify` with both native auxiliary-discovery implementations still selected.
- [x] 5.3 Run the canonical ESP32 physical matrix on the connected Android device, connected iOS device, and macOS host; require every non-location product checkpoint and defer only bounded location acquisition per the agreed manual follow-up. Android passed 17/17 and macOS passed 16/16. iOS passed all 16 foreground scenarios; its background XCTest passed twice and fixture telemetry continued throughout the background dwell, while the already-passed `xcodebuild` parent hit the known 120-second exit timeout. Bounded location sampling remains deferred.
