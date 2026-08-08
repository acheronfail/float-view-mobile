# Shared native backend

This workspace is the application and protocol owner shared by the Apple and
Android hosts. Dependencies point downward:

```text
host-c-abi / host-android-jni
              |
         host-runtime
              |
         app-backend -------- infra-loopback-server
              |                         |
       app-coordinator          infra-sqlite-storage
        /            \
host-ports       protocol-refloat / protocol-vesc-bms
                         \        /
                       protocol-vesc
                             |
                        domain-types
```

`app-backend` contains the device runtime plus focused `composition`,
`contract_mapping`, `settings`, and deterministic `software_host` modules used
by every host. Its `software-host` feature adds the deterministic command/event
adapter used by canonical conformance; normal native builds do not enable that
feature. `test-standalone` is only the deterministic executable launcher.
`host-runtime` is the bounded bridge and process lifecycle owner used by the C
ABI and JNI; it depends on `app-backend`, never on the launcher, and is not a
generic utility crate. Platform hosts implement `host-ports` and do not own
protocol, application, SQL, archive, or loopback semantics.

The public host boundary is the checked-in
`include/float_view_core.h`. Apple uses a hand-written Swift wrapper around its
opaque handle and one dedicated blocking command pump. Android explicitly
registers a small JNI facade and runs the same pull-based command lifecycle on
one Kotlin worker. Rust never calls back into Swift or Kotlin, and neither host
retains Rust memory. Runtime generations and operation identifiers reject stale
events and completions during lifecycle or connection replacement.

Host adapters own only OS work: Bluetooth byte transport, location,
background/lifecycle eligibility, immutable resource lookup, bounded document
staging, storage-root reporting, permissions/settings/theme/service state,
native export presentation, clocks/randomness, and privacy-safe diagnostics.
Application state, protocol routing, archive validation, SQLite, frontend
contract dispatch, and loopback authentication remain in Rust.

Cargo dependencies are locked by `Cargo.lock`. See `DEPENDENCIES.md` for direct
dependency ownership and license disposition, `TOOLCHAINS.md` for pinned setup
and target policy, `ABI.md` for interoperability invariants, and `SQLITE.md` for
the bundled engine identity. Normal host builds use `--locked --offline` and
consume the checked-in header; only `just generate core-header` invokes the
pinned generator. `just core-header-check` verifies byte-for-byte drift,
artifact hashes, C compilation, and ABI lifecycle behavior.

Bridge diagnostics are bounded stage/error descriptions. They must not contain
launch tokens, Bluetooth addresses or platform identifiers, coordinates, full
telemetry, SQLite rows, or unbounded payloads.

Fixture modules use `include_bytes!`/`include_str!` against the immutable files
under `backends/fixtures`. They compile only for protocol tests or the explicit
`fixtures` features selected by `test-software-adapters`; product feature graphs
do not include them. Tests never generate or rewrite canonical fixtures.
Protocol crates may depend on `domain-types` and narrower protocol crates, but
they must not depend on hosts, HTTP, SQLite, views, or bridge code.
