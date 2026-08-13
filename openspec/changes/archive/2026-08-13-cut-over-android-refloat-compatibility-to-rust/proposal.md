## Why

Apple now selects the bounded Rust Refloat compatibility component, while Android still duplicates INFO identity decoding, the exact supported-release catalogue, telemetry/control profile selection, and dynamic-layout admission in Kotlin. Selecting the same proven component on Android is the smallest reversible ownership transfer, completes this protocol-policy slice across every host, and removes the remaining native compatibility algorithms before higher-level application behavior moves to Rust.

## What Changes

- Add one temporary, narrow Android JNI bridge that exposes the existing canonical INFO request, compatibility record, and layout-admission decision through JVM-owned primitive arrays, fixed statuses, call-local validation, and panic containment.
- Build and package the bridge from locked source for the host JVM and all four Android ABIs through the existing shared bridge catalogue and typed Gradle registration.
- Replace Android INFO parsing, release/profile lookup, and required-layout policy with a strict Rust-record adapter while preserving existing models, protocol errors, controller-discovery effects, direct/CAN routing, scheduling, telemetry, controls, application behavior, persistence, and frontend behavior.
- Reuse the current-generation canonical Rust dynamic-layout record for compatibility admission instead of reconstructing or reserializing it from Kotlin field lists.
- Delete superseded Kotlin INFO offsets, compatibility catalogue/profile derivation, required-name lists, set-based layout admission, and every runtime fallback after the Rust path is selected.
- Preserve the selected Apple bridge and production behavior unchanged while proving exact JNI records, adversarial boundaries, clean packaging, complete software verification, and Android/iOS/macOS ESP32 regression.
- Treat both platform compatibility bridges as temporary migration scaffolding that will be removed when the local server, application behavior, protocols, and SQLite run behind one Rust runtime boundary.
- Non-goals: changing the safe compatibility component or canonical records; moving controller discovery, routing, scheduling, application behavior, SQLite, loopback transport, BLE, location, background execution, files, theme, lifecycle, permissions, WebView integration, or the frontend contract; consolidating bridges or introducing generated bindings.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-core-migration`: Make safe Rust the sole Android owner of Refloat INFO construction/decoding, exact compatibility/profile selection, and dynamic-layout admission while retaining higher-level Kotlin owners.
- `platform-hosts`: Define the temporary Android compatibility JNI bridge, strict Kotlin record adapter, locked packaging boundary, Apple isolation, and superseded Kotlin-algorithm deletion.
- `backend-conformance`: Require focused JNI/adapter, clean packaging, no-fallback, complete software, and all-host ESP32 evidence for the Android ownership transfer.

## Impact

This affects the locked `backends/core` workspace, one new `refloat-compatibility-android-jni` crate, shared bridge catalogue metadata, Android Gradle packaging, one Kotlin native loader/record adapter, focused Rust/JVM tests, root build/audit recipes, Android/core ownership documentation, and OpenSpec requirements. Apple production sources and artifacts, frontend APIs, SQLite schemas and data, OS-capability ownership, and user-visible behavior remain unchanged. One temporary JNI artifact is added now and explicitly deleted with the eventual unified Rust application runtime.
