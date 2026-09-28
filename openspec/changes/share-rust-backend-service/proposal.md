## Why

The shared loopback server removed a third frontend API implementation from a future Linux backend, but Swift and Kotlin still duplicate product persistence, archive handling and application orchestration. A new host should implement OS integration only, rather than reproduce settings, board identity, Tune rules, session recovery or reconnect policy.

## What Changes

- Select one process-owned Rust backend service on Apple and Android that composes the existing application runtime, shared server and shared product/session storage.
- Move settings, saved boards, profiles, routing hints, Tunes and schema-cache persistence, session catalog/recovery/deletion, maintenance orchestration, archive validation and CSV/archive generation into Rust.
- Move remaining portable application decisions into that service: saved/guest activation, board identity, session retention/close, reconnect and discovery timing, location capture eligibility, configuration/Tune request completion and publication ordering.
- Replace domain operations sent back to native application services with bounded OS facts/effects. Hosts retain Bluetooth and location acquisition, clocks/timer execution, permissions, background execution, storage-root/file-protection setup, native import/export presentation, theme and WebView integration.
- Delete superseded native stores, product records/projections, policy branches, domain-operation dispatch and redundant Rust-to-native-to-Rust conversions as each host selects the service. Keep the two existing binding artifact families.
- Preserve the public frontend contract, canonical SQL and archive format, existing identity/revision rules and bounded-resource behavior. Internal C/JNI envelopes may change together with both hosts; no public API or persistence-format break is intended.
- Apply the repository's pre-release policy: select one canonical board hash and reset incompatible development storage with marker `12`, without legacy identity aliases or migrations. SQL layout and current archive encoding stay unchanged; earlier development data need not be preserved.
- Prove the OS-only boundary using a direct Rust test host, deletion audits, canonical conformance, native packaging and the Android/iOS/macOS ESP32 checkpoint.

### Non-goals

Implementing or claiming support for Linux; adding a new frontend; database migrations, cloud sync or new product behavior; replacing existing domain kernels; generated bindings; a second runtime; permanent native fallbacks. Portable filesystem/SQLite work is shared, while OS-specific access, protection and presentation remain native.

## Capabilities

### New Capabilities

- `shared-rust-backend-service`: One complete portable backend service, product storage and orchestration with an OS-only host boundary.

### Modified Capabilities

- `platform-hosts`: Restrict selected hosts to OS execution and select shared activation/catalog ownership.
- `apple-unified-runtime-cutover`: Remove native product/catalog/application-policy ownership while retaining Apple capabilities.
- `android-unified-runtime-cutover`: Remove native product/catalog/application-policy ownership while retaining Android capabilities.
- `shared-rust-loopback-server`: Dispatch portable operations directly to the shared service, publish from shared state and move archive ownership out of native workers.
- `unified-rust-runtime`: Route portable storage/cache/activation effects to shared service composition; preserve the lower runtime's OS and transport independence.
- `backend-conformance`: Require direct-host proof, cross-host persistence parity, deletion audits and complete software/physical acceptance.

## Impact

Primary deletion targets are both `storage/SQLiteLogStore` implementations and portable portions of both `application/ApplicationService`, `RuntimeRecords`, `transport/ServerOperations` and `ServerFacts`. Update shared Rust composition/bindings, native startup and capability adapters, `backends/sql` ownership docs, conformance, build/symbol audits, AGENTS.md and OpenSpec context. Reuse the locked SQLite, JSON, hashing and randomness dependencies; justify any additional dependency against deleted implementation. Require a measured net reduction below the current 49,922 production lines and preserve existing convergence gates, allowing at most two additional cohesive safe crates. No implementation code is changed by this proposal.
