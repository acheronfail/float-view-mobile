# Shared Rust backend

One `backend-service::Process` owns the runtime, product store and maintenance
executor. Native hosts compose it through the existing C/JNI bindings. Listeners
borrow that process; replacing a WebView or listener must not restart recording.

## Owners

| Crate or module                                                               | Responsibility                                                     |
| ----------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| [backend-service](backend-service/src/lib.rs)                                 | Process composition and application policy                         |
| [product-store](product-store/src/lib.rs)                                     | Settings, boards, Tunes, drafts, catalog, recovery and archives    |
| [application-runtime](application-runtime/src/lib.rs)                         | Protocol coordination and the sole session writer                  |
| [loopback-server](loopback-server/src/lib.rs)                                 | Authenticated dispatch, public snapshots and bounded history reads |
| [application-runtime-host](application-runtime-host/src/lib.rs)               | Safe process registry and bounded result buffers                   |
| [application-runtime-apple-ffi](application-runtime-apple-ffi/src/lib.rs)     | Unsafe C buffer binding                                            |
| [application-runtime-android-jni](application-runtime-android-jni/src/lib.rs) | Unsafe, call-local JVM array binding                               |
| [application-runtime::load](application-runtime/src/load.rs)                  | Shared drive-load calculation                                      |
| [battery_estimator](backend-service/src/battery_estimator.rs)                 | Shared state-of-charge estimate                                    |
| [announcements](backend-service/src/announcements.rs)                         | Speech eligibility, timing and cancellation                        |

Protocol codecs and state machines remain downward dependencies, not additional
host runtimes. Native adapters execute OS effects; product dispatch, persistence,
reconnect decisions and snapshot construction stay shared.

## Runtime boundaries

- Transport workers reuse the existing application owner. Read-only workers use
  bounded independent readers and service-issued leases.
- An exclusive root lock prevents another process from recovering or deleting
  resources that are in use. Export presentation retains its lease until dismissal
  or completion of the scoped destination copy.
- The OS exchange validates lifetimes, generations, sequence and byte bounds.
  Cleanup must remain possible under queue pressure.
- Keep exactly the existing two unsafe binding families: caller-owned C buffers
  and call-local JVM arrays, with no retained JVM references or callbacks.
- Native build helpers use the pinned toolchain and locked offline dependencies,
  audit exported symbols and keep artifacts under `target/`.

`audit-ownership.sh` checks these ownership boundaries. Source and workspace
counts are informational; completed migration size ceilings are historical.
Run affected crate/consumer tests and direct conformance as described in the
[verification guide](../../docs/development.md#verification).

Storage compatibility and frozen fixtures are documented in
[storage-baseline.md](../../docs/storage-baseline.md). Battery catalog provenance
and selection rules are in [BATTERY_PRESETS.md](product-store/BATTERY_PRESETS.md).
Public behavior belongs in the [canonical specs](../../openspec/specs/), rather
than a second inventory of timings, thresholds or wire fields here.
