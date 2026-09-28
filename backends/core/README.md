# Shared Rust backend

`backend-service::Process` owns one application runtime, product store and
maintenance executor. Both native hosts select this constructor through the
existing C/JNI binding families. Listeners borrow the same service; stopping a
listener or replacing a WebView does not stop recording or maintenance.

`product-store` owns catalog startup, settings, board identity and profiles,
Tunes, schema caches, session lifecycle/recovery, archive validation and CSV.
`application-runtime` owns protocol coordination and the sole session writer.
Read-only history, archive and maintenance workers use bounded independent
readers and service-issued leases. An exclusive root lock prevents a second
process owner from recovering or deleting resources still in use.

`loopback-server` owns authenticated HTTP/WebSocket routes, public dispatch and
projection, bounded historical reads and compression. It calls the shared
service directly. There is no native domain-operation exchange, native catalog,
second runtime, generated API or per-protocol binding.

Native hosts supply the private root, HTML and closed OS facts; they execute
Bluetooth, timer, location, permission, lifecycle, theme and file-presentation
effects. The OS exchange validates lifetime, sequence, generations and byte
bounds. Queues, timers, workers, files and correlated requests have explicit
limits, with reserved cleanup capacity. Native presentation retains an export
lease until dismissal or completion of a scoped destination copy.

`application-runtime-host` is the safe process registry and bounded result
buffer. Exactly two unsafe bindings sit above it:

- `application-runtime-apple-ffi`: caller-owned C buffers for iOS/macOS;
- `application-runtime-android-jni`: call-local JVM arrays, without retained JNI
  references or callbacks.

The build scripts audit exact exported symbols and use the locked dependency
set. Android packages one library for each supported ABI. Generated output stays
under `target/`. The workspace contains at most 28 members; protocol codecs and
state machines remain downward dependencies, not separate host runtimes.

`audit-ownership.sh` checks production source below 49,922 lines, two binding
families, the workspace ceiling and absence of native portable implementations.
Use focused Cargo tests during development and `just verify` as the complete
software gate. Conformance includes a direct Rust host with synthetic OS facts
and the same canonical product scenarios used by native hosts. The connected
Android/iOS devices and ESP32 are required for the explicit physical checkpoint.

The application has no users or release version. Storage marker `12` resets
incompatible owned development data; ordinary corruption or I/O errors never
trigger a reset. Do not retain migrations, legacy identity aliases, previous
format readers or fallback implementations before the first release. See
AGENTS.md for the persistent policy.
