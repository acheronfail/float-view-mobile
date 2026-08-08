# Core direct dependencies

Cargo.lock is the identity authority. All registry dependencies are resolved
from crates.io and normal builds use `--locked`. The application team owns
updates: advance one responsibility at a time, inspect the lockfile/features
and upstream changelog/security advisories, rerun every target smoke, refresh
this table, and retain any newly required license or notice material.

| Crate               | Locked  | Responsibility                                                    | Target proof                                            | License disposition                                         |
| ------------------- | ------- | ----------------------------------------------------------------- | ------------------------------------------------------- | ----------------------------------------------------------- |
| `axum`              | 0.8.9   | loopback HTTP/WebSocket routing                                   | macOS run; iOS and four Android ABI builds; Android run | MIT; compatible, retain upstream license with distributions |
| `tokio`             | 1.53.1  | bounded async runtime and shutdown                                | same as Axum                                            | MIT; compatible, retain upstream license                    |
| `tower`             | 0.5.3   | concurrency and request bounds                                    | same as Axum                                            | MIT; compatible, retain upstream license                    |
| `futures-util`      | 0.3.33  | bounded WebSocket stream/sink operations                          | same as Axum                                            | MIT OR Apache-2.0; distribute under MIT and retain text     |
| `tokio-tungstenite` | 0.27.0  | loopback smoke client only                                        | macOS tests only; not in products                       | MIT; development-only                                       |
| `rusqlite`          | 0.37.0  | safe SQLite connection, query, WAL, and interrupt ownership       | macOS run; iOS and four Android ABI builds; Android run | MIT; compatible, retain upstream license                    |
| `serde`             | 1.0.228 | explicit internal persistence and loopback data mapping           | macOS run; iOS and four Android ABI builds; Android run | MIT OR Apache-2.0; distribute under MIT and retain text     |
| `serde_json`        | 1.0.149 | hand-written frontend contract and archive metadata serialization | macOS run; iOS and four Android ABI builds; Android run | MIT OR Apache-2.0; distribute under MIT and retain text     |
| `base64`            | 0.22.1  | bounded portable archive payload encoding                         | macOS run; iOS and four Android ABI builds; Android run | MIT OR Apache-2.0; distribute under MIT and retain text     |
| `csv`               | 1.4.0   | standards-compliant bounded session CSV export                    | macOS run; iOS and four Android ABI builds; Android run | MIT OR Unlicense; compatible, retain upstream license       |
| `jni`               | 0.21.1  | explicit Android JNI registration and call-scoped values          | four APK ABIs and physical instrumentation              | MIT OR Apache-2.0; distribute under MIT and retain text     |
| `sha2`              | 0.10.9  | SHA-256 fingerprints and hashes                                   | macOS/Android run; iOS and four Android ABI builds      | MIT OR Apache-2.0; distribute under MIT and retain text     |
| `getrandom`         | 0.3.4   | operating-system secure random bytes                              | macOS/Android run; iOS and four Android ABI builds      | MIT OR Apache-2.0; distribute under MIT and retain text     |
| `flate2`            | 1.1.9   | gzip and zlib-compatible streams without a system zlib            | macOS/Android run; iOS and four Android ABI builds      | MIT OR Apache-2.0; distribute under MIT and retain text     |
| `roxmltree`         | 0.21.1  | bounded read-only Refloat XML parsing                             | macOS/Android run; iOS and four Android ABI builds      | MIT OR Apache-2.0; distribute under MIT and retain text     |
| `crc32c`            | 0.6.8   | CRC32C wire signatures                                            | macOS/Android run; iOS and four Android ABI builds      | Apache-2.0 OR MIT; distribute under MIT and retain text     |
| `zip`               | 4.6.1   | bounded container-entry archive handling                          | macOS/Android run; iOS and four Android ABI builds      | MIT; compatible, retain upstream license                    |
| `tempfile`          | 3.27.0  | collision-resistant automatic test workspace cleanup              | macOS tests only                                        | MIT OR Apache-2.0; development-only                         |

The dependency smoke checks the standard SHA-256 and CRC32C vectors, obtains
32 bytes from the platform secure-random provider, creates a gzip stream,
decodes a zlib stream, parses XML, writes quoted CSV, and round-trips a size-bounded archive
entry. `flate2` has default features disabled and uses its Rust backend; SQLite
uses only the Cargo-bundled source described in `SQLITE.md`.

## Private host ABI provenance

ABI version 4 and build identity `float-view-abi04` are generated from
`host-c-abi` with pinned cbindgen 0.29.2. The checked-in header checksum,
Apple exported-symbol allowlist, Android JNI registration and packaging
allowlist are updated atomically. Startup inputs and focused event calls copy
bounded caller-owned buffers; no native software-transport dependency remains.
