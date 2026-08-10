## Why

Android still structurally decodes `COMM_FW_VERSION` in Kotlin even though the audited safe Rust decoder is already the production owner on Apple. Moving this one duplicate protocol boundary now completes cross-platform firmware-identity ownership without widening the change into compatibility, routing, Bluetooth, or application behavior.

## What Changes

- Add one focused Android JNI bridge crate over the existing safe Rust firmware-identity component, with locked offline builds, a fixed bounded record/status ABI, panic containment, and exact symbol/dependency audits.
- Replace Android's Kotlin VESC structural parser with a thin adapter that calls Rust, validates and maps the bounded result into the existing `FirmwareInfo`, and preserves established error classes.
- Package the source-built bridge for all four supported Android ABIs and load the exact host artifact for JVM tests.
- Delete the superseded Kotlin common-prefix, terminated-name, hardware-tail, and CRC-tail algorithms after focused, packaging, software, and physical verification succeeds.
- Rerun focused ESP32 identity regression on Android, iOS, and macOS before deletion is considered complete; a missing location sample remains deferred manual coverage.
- Preserve compatibility admission, route attribution, Refloat/BMS behavior, framing, BLE, background work, location, lifecycle, files, theme, WebViews, application coordination, SQLite, the frontend contract, and Apple bridge behavior as explicit non-goals.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Select the safe Rust firmware-identity decoder as Android's sole structural decoder and forbid a Kotlin fallback.
- `platform-hosts`: Define the narrow Android JNI ownership and keep OS/application capabilities outside the bridge.
- `backend-conformance`: Require focused bridge, packaging, software, source-deletion, and all-host physical evidence for the Android cutover.

## Impact

Affected areas are `backends/core` workspace membership and a new Android JNI bridge, Android Gradle native build/test packaging, the existing Kotlin firmware-identity decoder and tests, root verification/audits, and the three listed OpenSpec capabilities. The frontend/backend wire contract, SQLite schemas and data, public Kotlin domain models, native permissions, and Apple application artifacts do not change. The only new production dependency edge is the Android bridge to the existing safe identity crate plus the already locked minimal `jni-sys` binding; no generated binding or committed native binary is added.
