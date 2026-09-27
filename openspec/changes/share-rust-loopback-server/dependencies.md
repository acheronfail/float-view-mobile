# Server dependency selection

The unselected `float-view-loopback-server` owns the network stack above the
application runtime. Existing bindings will compose it during integration;
application-runtime and domain crates do not depend on it. The server forbids
unsafe Rust. Native production still selects FlyingFox/Ktor until cutover.

| Dependency         | Features                         | Replacement rationale                                                         |
| ------------------ | -------------------------------- | ----------------------------------------------------------------------------- |
| axum 0.8.9         | no defaults; http1, tokio, ws    | Route/upgrade handling over Hyper/Tungstenite replaces FlyingFox and Ktor     |
| tokio 1.53.1       | no defaults; rt, net, sync, time | Dedicated current-thread socket executor, bounded channels and deadlines      |
| flate2 1.1.9       | no defaults; rust_backend        | Shared gzip replaces native zlib/GZIPOutputStream without target C toolchains |
| getrandom 0.3.4    | OS backend                       | Fallible OS entropy for the launch secret                                     |
| time 0.3.53        | no defaults; std, formatting     | Shared RFC3339 location timestamps                                            |
| serde / serde_json | existing versions; derive        | Hand-written typed internal records and shared wire serialization             |

Network/compression/randomness versions are exact manifest pins; all dependencies
and checksums are locked. HTTP/2, TLS, filesystem serving, multipart,
form/query/JSON extractors, tracing and multi-thread runtime are not selected.
Axum's ws feature enables Tokio macros transitively. No new binding artifact.

References: [Axum](https://docs.rs/axum/0.8.9/axum/),
[flate2](https://docs.rs/flate2/1.1.9/flate2/),
[getrandom](https://docs.rs/getrandom/0.3.4/getrandom/).
Local Cargo.toml metadata supplies the license inventory below.

Lockfile delta: 62 external package/version entries, including platform-specific entries. This is not a linked-library count. flate2, serde and serde_json already existed.

| Added package     | Version                       | License                                             |
| ----------------- | ----------------------------- | --------------------------------------------------- |
| atomic-waker      | 1.1.2                         | Apache-2.0 OR MIT                                   |
| axum              | 0.8.9                         | MIT                                                 |
| axum-core         | 0.5.6                         | MIT                                                 |
| base64            | 0.22.1                        | MIT OR Apache-2.0                                   |
| block-buffer      | 0.10.4                        | MIT OR Apache-2.0                                   |
| bytes             | 1.12.1                        | MIT                                                 |
| cpufeatures       | 0.2.17                        | MIT OR Apache-2.0                                   |
| crypto-common     | 0.1.7                         | MIT OR Apache-2.0                                   |
| data-encoding     | 2.11.1                        | MIT                                                 |
| digest            | 0.10.7                        | MIT OR Apache-2.0                                   |
| futures-channel   | 0.3.34                        | MIT OR Apache-2.0                                   |
| futures-core      | 0.3.34                        | MIT OR Apache-2.0                                   |
| futures-sink      | 0.3.34                        | MIT OR Apache-2.0                                   |
| futures-task      | 0.3.34                        | MIT OR Apache-2.0                                   |
| futures-util      | 0.3.34                        | MIT OR Apache-2.0                                   |
| generic-array     | 0.14.7                        | MIT                                                 |
| getrandom         | 0.3.4                         | MIT OR Apache-2.0                                   |
| http              | 1.5.0                         | MIT OR Apache-2.0                                   |
| http-body         | 1.1.0                         | MIT                                                 |
| http-body-util    | 0.1.5                         | MIT                                                 |
| httparse          | 1.10.1                        | MIT OR Apache-2.0                                   |
| httpdate          | 1.0.3                         | MIT OR Apache-2.0                                   |
| hyper             | 1.11.0                        | MIT                                                 |
| hyper-util        | 0.1.20                        | MIT                                                 |
| log               | 0.4.34                        | MIT OR Apache-2.0                                   |
| matchit           | 0.8.4                         | MIT AND BSD-3-Clause                                |
| mime              | 0.3.17                        | MIT OR Apache-2.0                                   |
| mio               | 1.2.2                         | MIT                                                 |
| percent-encoding  | 2.3.2                         | MIT OR Apache-2.0                                   |
| pin-project-lite  | 0.2.17                        | Apache-2.0 OR MIT                                   |
| ppv-lite86        | 0.2.21                        | MIT OR Apache-2.0                                   |
| r-efi             | 5.3.0                         | MIT OR Apache-2.0 OR LGPL-2.1-or-later              |
| rand              | 0.9.5                         | MIT OR Apache-2.0                                   |
| rand_chacha       | 0.9.0                         | MIT OR Apache-2.0                                   |
| rand_core         | 0.9.5                         | MIT OR Apache-2.0                                   |
| sha1              | 0.10.7                        | MIT OR Apache-2.0                                   |
| slab              | 0.4.12                        | MIT                                                 |
| socket2           | 0.6.5                         | MIT OR Apache-2.0                                   |
| sync_wrapper      | 1.0.2                         | Apache-2.0                                          |
| thiserror         | 2.0.20                        | MIT OR Apache-2.0                                   |
| thiserror-impl    | 2.0.20                        | MIT OR Apache-2.0                                   |
| tokio             | 1.53.1                        | MIT                                                 |
| tokio-macros      | 2.7.2                         | MIT                                                 |
| tokio-tungstenite | 0.29.0                        | MIT                                                 |
| tower             | 0.5.3                         | MIT                                                 |
| tower-layer       | 0.3.3                         | MIT                                                 |
| tower-service     | 0.3.3                         | MIT                                                 |
| tungstenite       | 0.29.0                        | MIT OR Apache-2.0                                   |
| version_check     | 0.9.5                         | MIT/Apache-2.0                                      |
| wasi              | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasip2            | 1.0.4+wasi-0.2.12             | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| windows-link      | 0.2.1                         | MIT OR Apache-2.0                                   |
| windows-sys       | 0.61.2                        | MIT OR Apache-2.0                                   |
| wit-bindgen       | 0.57.1                        | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| zerocopy          | 0.8.57                        | BSD-2-Clause OR Apache-2.0 OR MIT                   |
| zerocopy-derive   | 0.8.57                        | BSD-2-Clause OR Apache-2.0 OR MIT                   |

| deranged | 0.5.8 | MIT OR Apache-2.0 |
| num-conv | 0.2.2 | MIT OR Apache-2.0 |
| powerfmt | 0.2.0 | MIT OR Apache-2.0 |
| time | 0.3.53 | MIT OR Apache-2.0 |
| time-core | 0.1.9 | MIT OR Apache-2.0 |
| time-macros | 0.2.31 | MIT OR Apache-2.0 |

## Compile evidence

`cargo build --locked --offline -p float-view-loopback-server --target TARGET`
passed for aarch64-apple-darwin, x86_64-apple-darwin, aarch64-apple-ios,
aarch64-apple-ios-sim, x86_64-apple-ios, aarch64-linux-android,
armv7-linux-androideabi, i686-linux-android and x86_64-linux-android.
The missing x86_64-apple-darwin Rust 1.93.0 target was installed.
These are candidate/dependency rlib builds, not linked production package checks.
Native linking, symbols and size deltas remain required at 4.3, 5.4, 6.4 and 7.2.

The existing rusqlite dependency enables `hooks` for cooperative cancellation.
Historical workers reuse the existing read-only SessionReader API without
constructing or locking a second ApplicationRuntime. Native code authorizes
the catalog entry and file path; Rust owns reading, projection and compression.
