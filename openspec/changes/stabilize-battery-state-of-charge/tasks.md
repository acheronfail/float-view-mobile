## 1. Contract and capacity

- [ ] 1.1 Add optional `batteryCapacityAh` validation/storage/archive support and `method`/`provisional` estimate fields in the authoritative contract, shared projection, mock, and affected Apple/Android consumers; verify boundary/null/invalid values, revision conflicts, archive round trips, and coherent consumer compilation in the same buildable slice.
- [ ] 1.2 Add independent deterministic charge and load-step fixtures using accepted monotonic timestamps; verify their expected Ah, original-axis SOC, and reserve-remapped values by arithmetic rather than estimator-generated outputs.

## 2. Shared temporal estimator

- [ ] 2.1 Replace current-only filtering with same-sample voltage/current curve observations and a fixed zero-load display-reserve mapping; verify source-knot oracles, interpolated loads, clamp behavior, regeneration observations, and extreme valid empty voltages.
- [ ] 2.2 Implement bounded voltage smoothing and provisional seeding; verify an initialized 80-to-30 observation step changes at most 0.1 displayed percentage points in one second, elapsed-time rather than sample-count behavior, and finite endpoint handling.
- [ ] 2.3 Implement signed trapezoidal charge integration when capacity/current are available, plus missing-current fallback; verify the 9 Ah/60 A/one-second oracle, irregular intervals, discharge/regen sign, zero current, charging, saturation, and no integration across broken pairs.
- [ ] 2.4 Implement sustained low-load/stable-voltage qualification and gradual correction with the design's initial constants; verify short releases do not recalibrate, renewed load stops correction, correction bounds hold after reserve remapping, and provisional status clears only on settled convergence.
- [ ] 2.5 Integrate the estimator with accepted/persisted sample ordering and connection ownership; verify duplicate/out-of-order and failed-persistence paths do not advance state, same-board gaps/reconnects resume provisionally without invented consumption, identity/configuration changes reseed, name-only changes do not reset, and WebView recreation has no effect.

## 3. Frontend and mock

- [ ] 3.1 Mirror temporal semantics in the mock against the independent fixtures and contract assertions; verify agreement with shared-service results within documented tolerances rather than using the mock as the numerical oracle.
- [ ] 3.2 Add optional full-pack Ah input and method/provisional help to shared battery configuration/detail surfaces; verify validation, serialized saves, primary/non-primary/keyboard behavior, and that blank capacity does not trigger missing-configuration fallback.
- [ ] 3.3 Reconcile with `add-battery-setup-fallback` if implemented, including the capacity field in its battery-only modal and preserving its reported-value selection; verify both live views still use one authoritative estimate and raw voltage/alerts remain responsive in browser/mock scenarios.

## 4. Conformance and verification

- [ ] 4.1 Extend canonical conformance with dynamic-load, regeneration, charging, rest, invalid data, persistence failure, lifecycle, and capacity cases; run affected direct/shared-server coverage and Apple/Android host contract integration coverage, reporting any unavailable host checks explicitly.
- [ ] 4.2 Replay available captured ride traces with acceleration, release, and rest alongside synthetic fixtures; document observed stability, provisional convergence, and any evidence-backed constant tuning. If no suitable captures/reference measurements exist, document that limitation and avoid absolute-accuracy claims.
- [ ] 4.3 Update frontend/backend documentation for capacity units, reserve mapping, method/provisional semantics, and limitations; verify consistency with all delta specs and remove superseded current-filter documentation.
- [ ] 4.4 Run focused product-store/backend-service and affected consumer tests, Rust formatting and relevant warnings-denied Clippy checks, frontend tests/type/lint checks, and affected battery browser scenarios; record results and expand only for failures or newly discovered impact. Validate the completed OpenSpec change before sync/archive.
