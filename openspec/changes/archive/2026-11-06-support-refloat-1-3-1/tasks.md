# Tasks

- [x] 1. Audit stable source and update the submodule; add exact stable admission while retaining beta2/beta3 and source-pinned shared fixtures.
- [x] 2. Verify stable admission, direct/CAN telemetry/configuration and retained beta behavior with the recorded software checks.
- [x] 3. Implement shared `1.3.1` Tune creation, existing-row reads/list pagination, update and safe application across audited beta2/beta3/stable; preserve exact configuration/session identity and schema safety.
- [x] 4. Verify persisted beta rows after reopen, family pagination, negative versions/schemas, frontend/mock policy, shared-service application and affected checks.
- [x] 5. Reconcile active compatibility planning, sync and validate canonical specs, then archive.

## Verification scope

The unchanged beta3/stable application source permits reuse of the existing beta3 physical checkpoint. The user explicitly requested closeout on that evidence; no repeated stable hardware check is required. The Tune-family change needs software coverage of compatibility and application safety, with no firmware, transport or native binding changes.
