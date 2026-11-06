# Verification

## Source and physical evidence

Stable source `v1.3.1` resolves to `a2b55d96f508653b0436b21410100aaf91f14b04`. Its complete diff from beta3 changes only `version` and three GitHub issue templates. Both `src/` trees have Git identity `63b5a05b513058ced93be53d363d945b5c4a3cbe`; `doc/` is also unchanged. The existing [beta3 production-host checkpoint](../../../../conformance/physical-checkpoints/refloat-1-3-1-beta3.md) therefore supplies the relevant physical evidence for unchanged wire behavior. Repeating the same hardware checks is not required for this release-identity or Tune-policy change. No new stable-firmware physical run is claimed.

Beta2 differs in calibration internals, while beta2/beta3/stable share identical configuration bytes, values and Tune semantics. The generator independently regenerated all releases and verified artifact equality before reuse. Stable adds manifest provenance without new fixture payload files. Full-schema fingerprint is `6bc6bfa433f8205519bc931723e39252aa71c9f538a0ed7c200d688486c63e73`, signature `0x191a6c1b`, with 172 serialized fields. Existing bounded parsing covers numeric stable suffix `1` and empty suffix as exact stable identity.

## Software results

- Initial release admission checks passed: 140 focused Rust tests across compatibility, configuration, mutation coordination, connection composition and telemetry; 27 simulator/fixture tests; both compatibility browser engines; frontend check/build and conformance TypeScript/fixture checks.
- Final Tune storage suites: all four focused Tune tests passed. Existing raw beta2/beta3 rows remain unchanged on read/reopen; public identities, family-filtered pages and cursors use `1.3.1`. Negative versions, fingerprints and signatures remain isolated. Metadata update retains revision behavior and original stored beta provenance.
- Final app transfer regression passed, including canonical export and import of a deliberately retained beta3 identity from a supported archive. No migration, storage marker or archive format change is needed.
- Final frontend Tune suite: 33 tests passed, including all nine beta2/beta3/stable overwrite combinations and negative identities. Shared mock-family pagination/read regression passed. Svelte check reported zero errors/warnings; production build passed with the existing bundle-size notice. Affected frontend ESLint passed.
- Final Tune browser suite: all four Chromium/WebKit tests passed, retaining touch/keyboard confirmation, comparison, overwrite, compatibility and error behavior.
- Final canonical production shared-service scenarios passed for stable, beta2 and beta3, each at revision 227. These exercise canonical Tune creation/list/get/update and guarded application to exact target firmware, verified readback, saved/guest reconnect, durable timeout history and exact observed board identities. One beta3 launch missed its process-start deadline; the clean retry passed.
- Final conformance TypeScript checks, affected all-targets warnings-denied Clippy, Rust formatting and changed-file formatting/diff checks passed. The production policy lives in shared Rust and uses existing C/JNI envelopes on both hosts; no native ABI or adapter change is needed.

Canonical spec validation passed with 27 existing requirement-length warnings; comparison with the pre-change texts confirmed every warned requirement description is unchanged. The release change passes strict validation with no issues.

## Closeout

The user requested reuse of unchanged beta3 firmware evidence and a shared audited `1.3.1` Tune family. Both are implemented and verified. The broader schema-admission proposal remains planning-only, reconciled to preserve the audited family exception. Specs are synced and the completed release change is archived after validation. No commits, pushes, hardware writes or online mutations were performed.
