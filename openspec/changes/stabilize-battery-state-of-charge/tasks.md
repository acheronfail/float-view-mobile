## 1. Shared preset catalog, contract, and capacity

Land tasks 1.1–1.3 as one coherent contract/catalog slice, with all consumers and documentation updated together; intermediate task progress must not ship a partial contract. This slice unlocks the companion UI.

- [x] 1.1 Implement the bounded shared preset catalog and snapshot metadata from design decision 2a, with stable variant IDs, provenance, exact/generic curve mapping, and nominal pack Ah. Verify every seed row, manufacturer variants, distinct XR/Pint X labels, duplicate/bounds checks, source arithmetic, unchanged existing curves, and 40S exclusion; document catalog ownership and Basic copy policy.
- [x] 1.2 Add optional `batteryCapacityAh` and `batteryPresetId` to profile validation/storage/archive and the authoritative contract; implement atomic service-side preset expansion, explicit re-selection, manual override/identity clearing, and S/P capacity invalidation. Verify boundaries, mixed-selection rejection, failed writes/conflicts, import consistency, identical-electrical presets, unknown IDs, and capacity clears/overrides. Document request versus import semantics.
- [x] 1.3 Update snapshot/profile contract projection, frontend client/mock, affected Apple/Android consumers, and canonical contract assertions together; verify catalog and preset/capacity round trips and compile consumers in a buildable slice. These tasks unlock the companion preset UI without depending on temporal tuning.
- [x] 1.4 Add independent deterministic charge and load-step fixtures using accepted monotonic timestamps; verify expected Ah, original-axis SOC, reserve-remapped values, and preset nominal capacity by arithmetic rather than estimator-generated outputs.

## 2. Shared temporal estimator

- [x] 2.1 Replace current-only filtering with same-sample voltage/current curve observations and a fixed zero-load display-reserve mapping; verify source-knot oracles, interpolated loads, clamp behavior, regeneration observations, and extreme valid empty voltages.
- [x] 2.2 Implement bounded voltage smoothing and provisional seeding; verify an initialized 80-to-30 observation step changes at most 0.1 displayed percentage points in one second, elapsed-time rather than sample-count behavior, and finite endpoint handling.
- [x] 2.3 Implement signed trapezoidal charge integration when capacity/current are available, plus missing-current fallback; verify the 9 Ah/60 A/one-second oracle, irregular intervals, discharge/regen sign, zero current, charging, saturation, and no integration across broken pairs.
- [x] 2.4 Implement sustained low-load/stable-voltage qualification and gradual correction with the design's initial constants; verify short releases do not recalibrate, renewed load stops correction, correction bounds hold after reserve remapping, and provisional status clears only on settled convergence.
- [x] 2.5 Integrate the estimator with accepted/persisted sample ordering and connection ownership; verify duplicate/out-of-order and failed-persistence paths do not advance state, same-board gaps/reconnects resume provisionally without invented consumption, effective configuration changes reseed once, preset-identity/name-only changes do not reset, and WebView recreation has no effect.
- [x] 2.6 Add `method`/`provisional` contract fields with shared projection, mock, affected Apple/Android consumers, and canonical assertions in the same buildable slice; verify truthful initial/uncertain/unavailable states and document temporal/reserve semantics alongside the estimator.

## 3. Frontend and mock

- [x] 3.1 Mirror temporal semantics in the mock against the independent fixtures and contract assertions; verify agreement with shared-service results within documented tolerances rather than using the mock as the numerical oracle.
- [x] 3.2 Add method/provisional help to battery details or Advanced help without technical curve/method requirements in Basic. Integrate the capacity input owned by the companion UI change; verify manual/nominal/null capacity, generic-profile charge integration, input accessibility, and that missing capacity or a missing dedicated curve does not trigger setup warnings. Document user-facing semantics with this slice.
- [x] 3.3 Integrate the companion Basic/Advanced setup with temporal estimation; verify presets reseed once on effective changes, mode switches and identity-only edits do not reset charge, cleared/overridden capacity survives reconnect/recreation, both live views use one authoritative estimate, and raw voltage/alerts remain responsive. Preserve reported fallback only for `not_configured`.

## 4. Conformance and verification

- [x] 4.1 Extend canonical conformance with dynamic-load, regeneration, charging, rest, invalid data, persistence failure, lifecycle, named/generic presets, atomic selection, manual overrides, and capacity cases; run affected direct/shared-server coverage and Apple/Android host contract integration coverage, reporting any unavailable host checks explicitly.
- [x] 4.2 Replay available captured ride traces with acceleration, release, and rest alongside synthetic fixtures; document observed stability, provisional convergence, and any evidence-backed constant tuning. If no suitable captures/reference measurements exist, document that limitation and avoid absolute-accuracy claims.
- [x] 4.3 Update frontend/backend documentation for capacity units, reserve mapping, method/provisional semantics, and limitations; verify consistency with all delta specs and remove superseded current-filter documentation.
- [x] 4.4 Run focused product-store/backend-service and affected consumer tests, Rust formatting and relevant warnings-denied Clippy checks, frontend tests/type/lint checks, and affected battery browser scenarios; record results and expand only for failures or newly discovered impact. Validate the completed OpenSpec change before sync/archive.

## Verification — catalog/contract slice (2026-09-30)

- 52 bounded variant presets with per-cell nominal arithmetic, unique IDs, known curve references, distinct XR/Pint X identities and unsupported 40S exclusion. Curves unchanged.
- Product-store tests pass, including atomic selection, revision conflicts, locked-store rejection, manual overrides/count invalidation, imports, archive round trips and reopening.
- Backend-service, loopback-server, Apple FFI and Android JNI crate tests pass. Local-socket tests required an elevated rerun after sandbox bind denial. Both hosts reuse the shared snapshot; no native preset implementation is added. Native application integration will also run with the final estimator slice.
- Canonical direct and mock conformance pass with catalog bounds, named/generic selections, failures, conflicts, overrides and reset assertions.
- Frontend focused mock tests: 23 passed; Svelte check: zero errors/warnings; conformance TypeScript check passed; changed frontend ESLint/Prettier and Rust formatting passed. Affected Rust Clippy with warnings denied passed.

## Verification — temporal estimator and integration (2026-09-30)

- Canonical independent JSON oracles and Rust temporal tests pass: signed charge accounting, original-axis reserve remapping, voltage-only load steps, gaps, charging/regeneration, rest qualification/convergence, unstable voltage, missing observations, saturation and extreme reserve bounds. Mock fixture tests match to 1e-12.
- Backend-service tests: 35 passed, including every accepted observation in a pre-publication batch, duplicate/reordered rejection, failed/conflicting configuration updates, identity-only preservation, capacity reseeding, reconnects and controller replacement. Existing runtime persistence-gate tests pass.
- Affected product-store, backend-service, loopback-server, application-runtime, Apple FFI and Android JNI test suites pass. Rust formatting and affected warnings-denied Clippy pass.
- Canonical software conformance passes for direct, mock, macOS, iOS simulator and the dedicated Android emulator. Android host tests were rerun with all 23 Gradle tasks executed using the installed SDK; Apple host tests pass (7 tests). No physical-device verification was used.
- Frontend regression suite: 227 passed. Final focused rerun: 15 passed. Svelte check has zero errors/warnings; frontend build, changed-file ESLint/Prettier and conformance TypeScript checks pass.
- Final Chromium/WebKit battery runs: all four scenarios pass, including Basic/Advanced draft preservation, explicit discard, ordinary-close flushing of hidden valid drafts, preset failures/reset, capacity clearing, recreation and both live percentage views.
- No suitable recorded ride captures/reference measurements were found in the repository. `conformance/README.md` records this limitation and the synthetic results; constants remain the specified engineering starting points. No absolute-accuracy improvement or empirical tuning is claimed.
