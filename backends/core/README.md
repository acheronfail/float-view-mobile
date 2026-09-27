# Shared Rust backend

`backends/core` is the locked Rust workspace shared by the native hosts. Safe
domain crates point downward into protocol codecs and deterministic state
machines. `application-runtime` composes them into the one selected application
owner used by Apple and Android.

## Selected boundary

The runtime owns:

- bounded VESC streams, framing, decoding, discovery, routing, and scheduling;
- Refloat telemetry, configuration, controls, Tunes, and mutation policy;
- auxiliary BMS discovery and live state;
- connection generations, timers, activation, recovery, and authoritative
  snapshots;
- canonical per-session SQLite writes, commit markers, reads, reduction,
  interruption, and pruning.

Hosts submit versioned bounded commands and facts and execute ordered effects.
The application runtime has no Bluetooth, location provider, clock, UI,
permission, lifecycle or native-file presentation implementation. Those remain
native OS responsibilities. The separate `loopback-server` crate owns HTTP and
WebSocket admission, public dispatch/projection, history reads and compression.
It runs one socket executor and two bounded historical workers, reusing the
existing read-only SessionReader without constructing another application runtime.
Native workers drain closed typed operations and publish complete domain facts
through the same C/JNI artifacts. Listener generations, launch token and public
revisions are independent of the application and disposable frontend lifetime. The frontend contract and canonical SQL schema remain shared
external contracts rather than generated bindings.

`application-runtime-host` provides the safe platform-neutral envelope,
registry, handles, tokens, pending results, interruption, statuses, and bounds.
It contains no raw host pointers or JNI operations. Exactly two unsafe binding
crates sit above it:

- `application-runtime-apple-ffi` validates the stable caller-owned C ABI used
  by the shared iOS/macOS adapter;
- `application-runtime-android-jni` validates call-local JNI inputs and
  caller-owned primitive arrays without retaining JVM references.

The bindings expose the same coarse lifecycle/command/result/interruption model.
They do not expose individual protocol crates, Rust allocation, callbacks, or
generated records. Adding Windows or Linux later should add a similarly thin
host binding or direct safe host integration, not fork the application runtime.

## Workspace organization

The remaining crates fall into four groups:

- protocol codecs and records (`vesc-*`, `refloat-*`);
- deterministic services (`*-discovery`, schedulers, telemetry,
  configuration, and mutation coordinators);
- persistence and history (`session-sqlite`, `session-history-kernel`);
- the selected composition and its two host bindings.

Domain crates remain independently tested because this keeps protocol and state
machine behavior small and reusable. They are implementation dependencies, not
separate production runtimes or host integration surfaces. There are no
per-protocol Apple FFI or Android JNI crates, no JNI support layer, no bridge
catalog, and no per-protocol native build matrix.

`build-apple-runtime.sh` produces the one Apple archive and audits its public
symbols. `build-android-runtime.sh` produces one host-test library plus one
library for each supported Android ABI and audits the six exact JNI exports.
Generated build output stays below `target/` and is never committed.

## Migration evidence and gates

`migration-baseline.txt` records the pre-runtime and cutover checkpoints.
`audit-migration-convergence.sh` measures tracked and proposed production source,
workspace members, bindings, temporary bridges, and Android target artifacts.
The final cutover must keep combined Rust, Apple, and Android production source
below 59,280 lines, workspace members below 34, exactly two runtime bindings,
and zero temporary protocol bridges.

Use the focused Cargo tests while changing a domain crate. Before committing a
host cutover, run warnings-denied workspace linting, both native test suites,
clean host/device/simulator/ABI packaging, software conformance, the convergence
audit, and the applicable physical fixture matrix.

## Invariants

- Safe algorithm crates forbid unsafe code.
- Unsafe code is confined to the named host binding crates and bundled SQLite
  boundary where required.
- Every input, collection, queue, retry, result, and retained observation has an
  explicit bound.
- Hosts own OS effects; Rust owns cross-platform decisions.
- There is one production application runtime and no native protocol fallback.
- New shared behavior extends the runtime or a downward dependency; it does not
  create sideways candidates, generated APIs, or another temporary bridge.
