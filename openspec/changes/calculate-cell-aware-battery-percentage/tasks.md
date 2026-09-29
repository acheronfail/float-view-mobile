# Tasks

## 1. Catalog and saved configuration

- [ ] 1.1 Add the pinned seven-profile catalog to shared Rust ownership with source attribution and version; verify every source knot, current level, finite value, and monotonic curve against `profile-data.json` using focused catalog tests.
- [ ] 1.2 Extend saved-profile validation for cell ID and minimum voltage without changing profile completeness; verify accepted boundaries, rejected unknown/nonfinite/out-of-range values, revision conflicts, and archive round trips with product-store tests.

## 2. Estimation and authoritative contract

- [ ] 2.1 Implement minimum remapping, load-knot interpolation, inverse lookup, series validation, and unavailable/mode selection; verify the independent design oracles plus endpoints, midpoints, all profiles, high-current clamp, missing current, regen, and invalid voltage in Rust tests.
- [ ] 2.2 Integrate constant-size time-based filtering into the existing shared application/service owner; verify sample ordering, filter step response, gaps, reconnects, board changes, configuration revisions, and persistence-failure publication gating with focused lifecycle tests.
- [ ] 2.3 Update `contract.ts`, shared projection, mock/client fixtures, and both Apple/Android host paths coherently with the bounded catalog and estimate fields; verify type checks and canonical conformance for complete snapshots, unavailable reasons, profile edits, guests, BMS-only devices, and unchanged raw telemetry/export meaning. Document units, nullability, and estimate provenance beside the contract.

## 3. Editor and live displays

- [ ] 3.1 Add saved-board cell-profile and empty-voltage controls with default/protection-setting explanation through existing autosave; verify persistence after reopening, validation feedback, revision conflicts, and primary-pointer/non-primary/keyboard activation in focused frontend tests.
- [ ] 3.2 Replace primary controller-percentage calculations in WidgetPower and DetailedView with the shared estimate and shared display rounding; verify disagreeing raw/estimated levels, identical percentages, gauge/color/accessibility behavior, unavailable and stale states, charging, and resolved per-cell voltage in frontend tests. Retain a separately labeled controller-reported detail row.
- [ ] 3.3 Verify WebView recreation and switching live/detailed views preserve the same estimate without extra telemetry polling or filter advancement using integration coverage; remove superseded primary percentage derivations and document the estimator's approximation and configuration behavior in the relevant frontend documentation.

## 4. Integration gate

- [ ] 4.1 Run focused cross-platform canonical conformance and frontend checks covering the complete configured-board flow and failure/reconnect scenarios; verify both hosts expose identical shared estimation behavior and resolve failures.
- [ ] 4.2 Run affected Rust crate/consumer tests and relevant type, lint and build checks alongside task 4.1; resolve failures and retain verification results for implementation and spec sync/archive. Use `just verify` only if broader impact warrants it; no physical checkpoint is needed for this shared calculation and display change.
